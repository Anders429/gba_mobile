use super::{
    super::{
        Payload, Response, Timeout, error, payload,
        payload::{ReceiveData, ReceiveLength, ReceiveParsed},
    },
    ReceiveError, receive_error,
};
use crate::{
    driver::{Adapter, frames},
    mmio::serial::SIODATA32,
};
use core::num::{NonZeroU8, NonZeroU16};
use either::Either;

#[derive(Debug)]
enum Step<Payload>
where
    Payload: self::Payload,
{
    HeaderLength { payload: Payload::ReceiveLength },
    Data { payload: Payload::ReceiveData },
    Checksum { payload: Payload::ReceiveParsed },
    Footer { payload: Payload::ReceiveParsed },
}

#[derive(Debug)]
struct State {
    command_xor: bool,
    checksum: u16,
    attempt: u8,
    frame: u8,
}

impl State {
    fn new(attempt: u8, checksum: u16, command_xor: bool) -> Self {
        Self {
            command_xor,
            checksum,
            attempt,
            frame: 0,
        }
    }

    fn next(self) -> Self {
        Self {
            command_xor: self.command_xor,
            checksum: self.checksum,
            attempt: self.attempt,
            frame: 0,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active) struct Receive<Payload>
where
    Payload: self::Payload,
{
    step: Step<Payload>,
    state: State,
}

impl<Payload> Receive<Payload>
where
    Payload: self::Payload,
{
    pub(super) fn new(
        payload: Payload::ReceiveLength,
        attempt: u8,
        checksum: u16,
        command_xor: bool,
    ) -> Self {
        Self {
            step: Step::HeaderLength { payload },
            state: State::new(attempt, checksum, command_xor),
        }
    }

    fn next(step: Step<Payload>, state: State) -> Self {
        Self {
            step,
            state: state.next(),
        }
    }
}

impl<Payload> super::super::Receive<Payload> for Receive<Payload>
where
    Payload: self::Payload,
{
    type ReceiveError = ReceiveError<Payload>;

    fn vblank(mut self) -> Result<Self, Timeout> {
        if self.state.frame > frames::THREE_SECONDS {
            return Err(Timeout::Serial);
        } else {
            self.state.frame += 1;
            Ok(self)
        }
    }

    fn timer(&self) {
        let bytes = match &self.step {
            Step::Footer { payload } => {
                let command_byte = if self.state.command_xor {
                    payload.command() as u8 | 0x80
                } else {
                    payload.command() as u8
                };
                u32::from_be_bytes([0x81, command_byte, 0x00, 0x00])
            }
            _ => 0x4b_4b_4b_4b,
        };
        unsafe { SIODATA32.write_volatile(bytes) };
    }

    fn serial(
        mut self,
    ) -> Result<Either<Result<Self, Self::ReceiveError>, Response<Payload>>, error::Receive<Payload>>
    {
        let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };
        match self.step {
            Step::HeaderLength { payload } => {
                if bytes[0] > 0 {
                    let full_length = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                    Ok(Either::Left(Err(ReceiveError::new(
                        receive_error::Step::Data {
                            index: 2,
                            length: unsafe { NonZeroU16::new_unchecked(full_length) },
                        },
                        payload.restart(),
                        error::Receive::LengthTooLarge(full_length),
                        self.state.attempt,
                    ))))
                } else {
                    match payload.receive_length(bytes[1]) {
                        Ok(Either::Left(payload)) => {
                            // Receive the last two bytes as data.
                            self.state.checksum = self
                                .state
                                .checksum
                                .wrapping_add(bytes[0] as u16)
                                .wrapping_add(bytes[1] as u16)
                                .wrapping_add(bytes[2] as u16)
                                .wrapping_add(bytes[3] as u16);
                            match payload.receive_data(bytes[2]) {
                                Ok(Either::Left(payload)) => {
                                    match payload.receive_data(bytes[3]) {
                                        Ok(Either::Left(payload)) => Ok(Either::Left(Ok(
                                            Self::next(Step::Data { payload }, self.state),
                                        ))),
                                        Ok(Either::Right(payload)) => Ok(Either::Left(Ok(
                                            Self::next(Step::Checksum { payload }, self.state),
                                        ))),
                                        Err((error, payload, Some((length, index)))) => {
                                            Ok(Either::Left(Err(ReceiveError::new(
                                                receive_error::Step::Data { index, length },
                                                payload,
                                                error::Receive::Payload(
                                                    payload::Error::ReceiveData(error),
                                                ),
                                                self.state.attempt,
                                            ))))
                                        }
                                        Err((error, payload, None)) => {
                                            Ok(Either::Left(Err(ReceiveError::new(
                                                receive_error::Step::Checksum,
                                                payload,
                                                error::Receive::Payload(
                                                    payload::Error::ReceiveData(error),
                                                ),
                                                self.state.attempt,
                                            ))))
                                        }
                                    }
                                }
                                Ok(Either::Right(payload)) => Ok(Either::Left(Ok(Self::next(
                                    Step::Checksum { payload },
                                    self.state,
                                )))),
                                Err((error, payload, Some((length, index)))) => {
                                    Ok(Either::Left(Err(ReceiveError::new(
                                        receive_error::Step::Data { index, length },
                                        payload,
                                        error::Receive::Payload(payload::Error::ReceiveData(error)),
                                        self.state.attempt,
                                    ))))
                                }
                                Err((error, payload, None)) => {
                                    Ok(Either::Left(Err(ReceiveError::new(
                                        receive_error::Step::Checksum,
                                        payload,
                                        error::Receive::Payload(payload::Error::ReceiveData(error)),
                                        self.state.attempt,
                                    ))))
                                }
                            }
                        }
                        Ok(Either::Right(payload)) => {
                            // No data to receive, so we move right on to checksum.
                            let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                            if full_checksum == self.state.checksum {
                                Ok(Either::Left(Ok(Self::next(
                                    Step::Footer { payload },
                                    self.state,
                                ))))
                            } else {
                                Ok(Either::Left(Err(ReceiveError::new(
                                    receive_error::Step::Footer,
                                    payload.restart(),
                                    error::Receive::Checksum {
                                        calculated: self.state.checksum,
                                        received: full_checksum,
                                    },
                                    self.state.attempt,
                                ))))
                            }
                        }
                        Err((error, payload)) => {
                            if let Some(nonzero_length) = NonZeroU16::new(bytes[1] as u16) {
                                if nonzero_length.get() > 2 {
                                    Ok(Either::Left(Err(ReceiveError::new(
                                        receive_error::Step::Data {
                                            length: nonzero_length,
                                            index: 2,
                                        },
                                        payload,
                                        error::Receive::Payload(payload::Error::ReceiveLength(
                                            error,
                                        )),
                                        self.state.attempt,
                                    ))))
                                } else {
                                    Ok(Either::Left(Err(ReceiveError::new(
                                        receive_error::Step::Checksum,
                                        payload,
                                        error::Receive::Payload(payload::Error::ReceiveLength(
                                            error,
                                        )),
                                        self.state.attempt,
                                    ))))
                                }
                            } else {
                                Ok(Either::Left(Err(ReceiveError::new(
                                    receive_error::Step::Footer,
                                    payload,
                                    error::Receive::Payload(payload::Error::ReceiveLength(error)),
                                    self.state.attempt,
                                ))))
                            }
                        }
                    }
                }
            }
            Step::Data { payload } => {
                self.state.checksum = self
                    .state
                    .checksum
                    .wrapping_add(bytes[0] as u16)
                    .wrapping_add(bytes[1] as u16);
                match payload.receive_data(bytes[0]) {
                    Ok(Either::Left(payload)) => match payload.receive_data(bytes[1]) {
                        Ok(Either::Left(payload)) => {
                            self.state.checksum = self
                                .state
                                .checksum
                                .wrapping_add(bytes[2] as u16)
                                .wrapping_add(bytes[3] as u16);
                            match payload.receive_data(bytes[2]) {
                                Ok(Either::Left(payload)) => {
                                    match payload.receive_data(bytes[3]) {
                                        Ok(Either::Left(payload)) => Ok(Either::Left(Ok(
                                            Self::next(Step::Data { payload }, self.state),
                                        ))),
                                        Ok(Either::Right(payload)) => Ok(Either::Left(Ok(
                                            Self::next(Step::Checksum { payload }, self.state),
                                        ))),
                                        Err((error, payload, Some((length, index)))) => {
                                            Ok(Either::Left(Err(ReceiveError::new(
                                                receive_error::Step::Data { index, length },
                                                payload,
                                                error::Receive::Payload(
                                                    payload::Error::ReceiveData(error),
                                                ),
                                                self.state.attempt,
                                            ))))
                                        }
                                        Err((error, payload, None)) => {
                                            Ok(Either::Left(Err(ReceiveError::new(
                                                receive_error::Step::Checksum,
                                                payload,
                                                error::Receive::Payload(
                                                    payload::Error::ReceiveData(error),
                                                ),
                                                self.state.attempt,
                                            ))))
                                        }
                                    }
                                }
                                Ok(Either::Right(payload)) => Ok(Either::Left(Ok(Self::next(
                                    Step::Checksum { payload },
                                    self.state,
                                )))),
                                Err((error, payload, Some((length, index)))) => {
                                    Ok(Either::Left(Err(ReceiveError::new(
                                        receive_error::Step::Data { index, length },
                                        payload,
                                        error::Receive::Payload(payload::Error::ReceiveData(error)),
                                        self.state.attempt,
                                    ))))
                                }
                                Err((error, payload, None)) => {
                                    Ok(Either::Left(Err(ReceiveError::new(
                                        receive_error::Step::Checksum,
                                        payload,
                                        error::Receive::Payload(payload::Error::ReceiveData(error)),
                                        self.state.attempt,
                                    ))))
                                }
                            }
                        }
                        Ok(Either::Right(payload)) => {
                            // The checksum is contained in the last two bytes.
                            let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                            if full_checksum == self.state.checksum {
                                Ok(Either::Left(Ok(Self::next(
                                    Step::Footer { payload },
                                    self.state,
                                ))))
                            } else {
                                Ok(Either::Left(Err(ReceiveError::new(
                                    receive_error::Step::Footer,
                                    payload.restart(),
                                    error::Receive::Checksum {
                                        calculated: self.state.checksum,
                                        received: full_checksum,
                                    },
                                    self.state.attempt,
                                ))))
                            }
                        }
                        Err((error, payload, Some((length, index)))) => {
                            Ok(Either::Left(Err(ReceiveError::new(
                                receive_error::Step::Data { index, length },
                                payload,
                                error::Receive::Payload(payload::Error::ReceiveData(error)),
                                self.state.attempt,
                            ))))
                        }
                        Err((error, payload, None)) => Ok(Either::Left(Err(ReceiveError::new(
                            receive_error::Step::Checksum,
                            payload,
                            error::Receive::Payload(payload::Error::ReceiveData(error)),
                            self.state.attempt,
                        )))),
                    },
                    Ok(Either::Right(payload)) => {
                        // The checksum is contained in the last two bytes.
                        let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                        if full_checksum == self.state.checksum {
                            Ok(Either::Left(Ok(Self::next(
                                Step::Footer { payload },
                                self.state,
                            ))))
                        } else {
                            Ok(Either::Left(Err(ReceiveError::new(
                                receive_error::Step::Footer,
                                payload.restart(),
                                error::Receive::Checksum {
                                    calculated: self.state.checksum,
                                    received: full_checksum,
                                },
                                self.state.attempt,
                            ))))
                        }
                    }
                    Err((error, payload, Some((length, index)))) => {
                        Ok(Either::Left(Err(ReceiveError::new(
                            receive_error::Step::Data { index, length },
                            payload,
                            error::Receive::Payload(payload::Error::ReceiveData(error)),
                            self.state.attempt,
                        ))))
                    }
                    Err((error, payload, None)) => Ok(Either::Left(Err(ReceiveError::new(
                        receive_error::Step::Checksum,
                        payload,
                        error::Receive::Payload(payload::Error::ReceiveData(error)),
                        self.state.attempt,
                    )))),
                }
            }
            Step::Checksum { payload } => {
                // The checksum is contained in the last two bytes.
                self.state.checksum = self
                    .state
                    .checksum
                    .wrapping_add(bytes[0] as u16)
                    .wrapping_add(bytes[1] as u16);
                let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                if full_checksum == self.state.checksum {
                    Ok(Either::Left(Ok(Self::next(
                        Step::Footer { payload },
                        self.state,
                    ))))
                } else {
                    Ok(Either::Left(Err(ReceiveError::new(
                        receive_error::Step::Footer,
                        payload.restart(),
                        error::Receive::Checksum {
                            calculated: self.state.checksum,
                            received: full_checksum,
                        },
                        self.state.attempt,
                    ))))
                }
            }
            Step::Footer { payload } => {
                match Adapter::try_from(bytes[0]) {
                    Ok(adapter) => match NonZeroU8::new(bytes[1]) {
                        None => {
                            // We don't care about what the adapter was set to previously. We just
                            // want to store whatever type it's currently telling us it is.
                            Ok(Either::Right(Response { payload, adapter }))
                        }
                        Some(nonzero) => {
                            // We can no longer retry at this point. We simply return the error.
                            Err(error::Receive::NonZeroFooterCommand(nonzero))
                        }
                    },
                    Err(unknown) => {
                        // We can no longer retry at this point. We simply return the error.
                        Err(error::Receive::UnsupportedDevice(unknown))
                    }
                }
            }
        }
    }
}
