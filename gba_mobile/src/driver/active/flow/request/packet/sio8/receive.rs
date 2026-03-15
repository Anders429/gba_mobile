use core::num::{NonZeroU8, NonZeroU16};

use super::{
    super::{
        Payload, Response, Timeout, error, payload,
        payload::{ReceiveCommand, ReceiveData, ReceiveLength, ReceiveParsed},
    },
    ReceiveError, receive_error,
};
use crate::{
    driver::{Adapter, Command, frames},
    mmio::serial::SIODATA8,
};
use either::Either;

#[derive(Debug)]
enum Step<Payload>
where
    Payload: self::Payload,
{
    MagicByte2 {
        payload: Payload::ReceiveCommand,
    },

    HeaderCommand {
        payload: Payload::ReceiveCommand,
    },
    HeaderEmptyByte {
        payload: Payload::ReceiveLength,
    },
    HeaderLength1 {
        payload: Payload::ReceiveLength,
    },
    HeaderLength2 {
        payload: Payload::ReceiveLength,
        first_byte: u8,
    },

    Data {
        payload: Payload::ReceiveData,
    },

    Checksum1 {
        payload: Payload::ReceiveParsed,
    },
    Checksum2 {
        payload: Payload::ReceiveParsed,
        first_byte: u8,
    },

    FooterDevice {
        payload: Payload::ReceiveParsed,
    },
    FooterCommand {
        payload: Payload::ReceiveParsed,
        adapter: Adapter,
    },
}

#[derive(Debug)]
struct State {
    command_xor: bool,
    checksum: u16,
    attempt: u8,
    frame: u8,
}

impl State {
    fn new(attempt: u8) -> Self {
        Self {
            command_xor: false,
            checksum: 0,
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
pub(in super::super) struct Receive<Payload>
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
    pub(super) fn new(payload: Payload::ReceiveCommand, attempt: u8) -> Self {
        Self {
            step: Step::MagicByte2 { payload },
            state: State::new(attempt),
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
        let byte = match &self.step {
            Step::FooterDevice { .. } => 0x81,
            Step::FooterCommand { payload, .. } => {
                if self.state.command_xor {
                    payload.command() as u8 | 0x80
                } else {
                    payload.command() as u8
                }
            }
            _ => 0x4b,
        };
        unsafe { SIODATA8.write_volatile(byte) };
    }

    fn serial(
        mut self,
    ) -> Result<Either<Result<Self, Self::ReceiveError>, Response<Payload>>, error::Receive<Payload>>
    {
        let byte = unsafe { SIODATA8.read_volatile() };
        log::debug!("received byte: {byte:#04x}");
        match self.step {
            Step::MagicByte2 { payload } => match byte {
                0x66 => Ok(Either::Left(Ok(Self::next(
                    Step::HeaderCommand { payload },
                    self.state,
                )))),
                _ => Ok(Either::Left(Err(ReceiveError::new(
                    receive_error::Step::HeaderCommand,
                    payload,
                    error::Receive::MagicValue2(byte),
                    self.state.attempt,
                )))),
            },
            Step::HeaderCommand { payload } => {
                self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                self.state.command_xor = byte & 0x80 == 0;
                match Command::try_from(byte & 0x7f) {
                    Ok(command) => match payload.receive_command(command) {
                        Ok(payload) => Ok(Either::Left(Ok(Self::next(
                            Step::HeaderEmptyByte { payload },
                            self.state,
                        )))),
                        Err((error, payload)) => Ok(Either::Left(Err(ReceiveError::new(
                            receive_error::Step::HeaderEmptyByte,
                            payload,
                            error::Receive::Payload(payload::Error::ReceiveCommand(error)),
                            self.state.attempt,
                        )))),
                    },
                    Err(unknown) => Ok(Either::Left(Err(ReceiveError::new(
                        receive_error::Step::HeaderEmptyByte,
                        payload,
                        error::Receive::UnknownCommand(unknown),
                        self.state.attempt,
                    )))),
                }
            }
            Step::HeaderEmptyByte { payload } => {
                self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                Ok(Either::Left(Ok(Self::next(
                    Step::HeaderLength1 { payload },
                    self.state,
                ))))
            }
            Step::HeaderLength1 { payload } => {
                self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                Ok(Either::Left(Ok(Self::next(
                    Step::HeaderLength2 {
                        payload,
                        first_byte: byte,
                    },
                    self.state,
                ))))
            }
            Step::HeaderLength2 {
                payload,
                first_byte,
            } => {
                self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                if first_byte > 0 {
                    let full_length = ((first_byte as u16) << 8) | (byte as u16);
                    Ok(Either::Left(Err(ReceiveError::new(
                        receive_error::Step::Data {
                            index: 0,
                            length: unsafe { NonZeroU16::new_unchecked(full_length) },
                        },
                        payload.restart(),
                        error::Receive::LengthTooLarge(full_length),
                        self.state.attempt,
                    ))))
                } else {
                    match payload.receive_length(byte) {
                        Ok(Either::Left(payload)) => Ok(Either::Left(Ok(Self::next(
                            Step::Data { payload },
                            self.state,
                        )))),
                        Ok(Either::Right(payload)) => Ok(Either::Left(Ok(Self::next(
                            Step::Checksum1 { payload },
                            self.state,
                        )))),
                        Err((error, payload)) => {
                            if let Some(nonzero_length) = NonZeroU16::new(byte as u16) {
                                Ok(Either::Left(Err(ReceiveError::new(
                                    receive_error::Step::Data {
                                        index: 0,
                                        length: nonzero_length,
                                    },
                                    payload,
                                    error::Receive::Payload(payload::Error::ReceiveLength(error)),
                                    self.state.attempt,
                                ))))
                            } else {
                                Ok(Either::Left(Err(ReceiveError::new(
                                    receive_error::Step::Checksum1,
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
                self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                match payload.receive_data(byte) {
                    Ok(Either::Left(payload)) => Ok(Either::Left(Ok(Self::next(
                        Step::Data { payload },
                        self.state,
                    )))),
                    Ok(Either::Right(payload)) => Ok(Either::Left(Ok(Self::next(
                        Step::Checksum1 { payload },
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
                    Err((error, payload, None)) => Ok(Either::Left(Err(ReceiveError::new(
                        receive_error::Step::Checksum1,
                        payload,
                        error::Receive::Payload(payload::Error::ReceiveData(error)),
                        self.state.attempt,
                    )))),
                }
            }
            Step::Checksum1 { payload } => Ok(Either::Left(Ok(Self::next(
                Step::Checksum2 {
                    payload,
                    first_byte: byte,
                },
                self.state,
            )))),
            Step::Checksum2 {
                payload,
                first_byte,
            } => {
                let full_checksum = ((first_byte as u16) << 8) | (byte as u16);
                if full_checksum == self.state.checksum {
                    Ok(Either::Left(Ok(Self::next(
                        Step::FooterDevice { payload },
                        self.state,
                    ))))
                } else {
                    Ok(Either::Left(Err(ReceiveError::new(
                        receive_error::Step::FooterDevice,
                        payload.restart(),
                        error::Receive::Checksum {
                            calculated: self.state.checksum,
                            received: full_checksum,
                        },
                        self.state.attempt,
                    ))))
                }
            }
            Step::FooterDevice { payload } => match Adapter::try_from(byte) {
                Ok(adapter) => Ok(Either::Left(Ok(Self::next(
                    Step::FooterCommand { payload, adapter },
                    self.state,
                )))),
                Err(unknown) => Ok(Either::Left(Err(ReceiveError::new(
                    receive_error::Step::FooterCommand,
                    payload.restart(),
                    error::Receive::UnsupportedDevice(unknown),
                    self.state.attempt,
                )))),
            },
            Step::FooterCommand { payload, adapter } => {
                // The acknowledgement signal command we receive is expected to be 0x00.
                if let Some(nonzero) = NonZeroU8::new(byte) {
                    // We can no longer retry at this point. We simply enter an error state.
                    Err(error::Receive::NonZeroFooterCommand(nonzero))
                } else {
                    // We don't care about what the adapter was set to previously. We just want to
                    // store whatever type it's currently telling us it is.
                    Ok(Either::Right(Response { payload, adapter }))
                }
            }
        }
    }
}
