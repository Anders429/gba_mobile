use super::{
    super::{Data, Timeout, communication, error, schedule_serial},
    ReceiveError, receive_error,
};
use crate::{
    driver::{Adapter, frames},
    mmio::serial::{SIODATA32, TransferLength},
};
use core::{
    cmp,
    num::{NonZeroU8, NonZeroU16},
};
use either::Either;

#[derive(Debug)]
enum Step {
    HeaderLength,
    Data { index: u8, length: NonZeroU8 },
    Checksum,
    Footer,
}

#[derive(Debug)]
struct State {
    command_xor: bool,
    checksum: u16,
    attempt: u8,
    frame: u8,
    communication_state: communication::State,
}

impl State {
    fn new(attempt: u8, checksum: u16, command_xor: bool) -> Self {
        Self {
            command_xor,
            checksum,
            attempt,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }

    fn next(self) -> Self {
        Self {
            command_xor: self.command_xor,
            checksum: self.checksum,
            attempt: self.attempt,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active) struct Receive {
    step: Step,
    state: State,
}

impl Receive {
    pub(super) fn new(attempt: u8, checksum: u16, command_xor: bool) -> Self {
        Self {
            step: Step::HeaderLength,
            state: State::new(attempt, checksum, command_xor),
        }
    }

    fn next(step: Step, state: State) -> Self {
        Self {
            step,
            state: state.next(),
        }
    }
}

impl super::super::Receive for Receive {
    type ReceiveError = ReceiveError;

    fn vblank(&mut self) -> Result<(), Timeout> {
        if self.state.frame > frames::THREE_SECONDS {
            return Err(Timeout::Serial);
        } else {
            self.state.frame += 1;
            Ok(())
        }
    }

    fn timer(&mut self, data: &Data) {
        if matches!(self.state.communication_state, communication::State::Send) {
            let bytes = match &self.step {
                Step::Footer => {
                    let command_byte = if self.state.command_xor {
                        data.command as u8 | 0x80
                    } else {
                        data.command as u8
                    };
                    u32::from_be_bytes([0x81, command_byte, 0x00, 0x00])
                }
                _ => 0x4b_4b_4b_4b,
            };

            self.state.communication_state = communication::State::Receive;
            unsafe { SIODATA32.write_volatile(bytes) };
            schedule_serial(TransferLength::_32Bit);
        }
    }

    fn serial(
        mut self,
        data: &mut Data,
    ) -> Result<Either<Result<Self, Self::ReceiveError>, Adapter>, error::Receive> {
        match self.state.communication_state {
            communication::State::Send => Ok(Either::Left(Ok(self))),
            communication::State::Receive => {
                let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };
                match self.step {
                    Step::HeaderLength => {
                        if bytes[0] > 0 {
                            let full_length = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                            Ok(Either::Left(Err(ReceiveError::new(
                                receive_error::Step::Data {
                                    index: 2,
                                    length: unsafe { NonZeroU16::new_unchecked(full_length) },
                                },
                                error::Receive::LengthTooLarge(full_length),
                                self.state.attempt,
                            ))))
                        } else if let Some(nonzero_length) = NonZeroU8::new(bytes[1]) {
                            // Receive the last two bytes as data.
                            self.state.checksum = self
                                .state
                                .checksum
                                .wrapping_add(bytes[0] as u16)
                                .wrapping_add(bytes[1] as u16)
                                .wrapping_add(bytes[2] as u16)
                                .wrapping_add(bytes[3] as u16);
                            if nonzero_length.get() > 2 {
                                unsafe {
                                    data.data.try_push(bytes[2]).unwrap_unchecked();
                                    data.data.try_push(bytes[3]).unwrap_unchecked();
                                }
                                Ok(Either::Left(Ok(Self::next(
                                    Step::Data {
                                        index: 2,
                                        length: nonzero_length,
                                    },
                                    self.state,
                                ))))
                            } else {
                                unsafe {
                                    data.data.try_push(bytes[2]).unwrap_unchecked();
                                    if nonzero_length.get() == 2 {
                                        data.data.try_push(bytes[3]).unwrap_unchecked();
                                    }
                                }
                                Ok(Either::Left(Ok(Self::next(Step::Checksum, self.state))))
                            }
                        } else {
                            // No data to receive, so we move right on to checksum.
                            let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                            if full_checksum == self.state.checksum {
                                Ok(Either::Left(Ok(Self::next(Step::Footer, self.state))))
                            } else {
                                Ok(Either::Left(Err(ReceiveError::new(
                                    receive_error::Step::Footer,
                                    error::Receive::Checksum {
                                        calculated: self.state.checksum,
                                        received: full_checksum,
                                    },
                                    self.state.attempt,
                                ))))
                            }
                        }
                    }
                    Step::Data { index, length } => {
                        self.state.checksum = self
                            .state
                            .checksum
                            .wrapping_add(bytes[0] as u16)
                            .wrapping_add(bytes[1] as u16);

                        let bytes_to_receive = length.get().saturating_sub(index);
                        for index in 0..cmp::min(4, bytes_to_receive) {
                            unsafe {
                                data.data.try_push(bytes[index as usize]).unwrap_unchecked();
                            }
                        }
                        if bytes_to_receive <= 2 {
                            // Checksum is included in last two bytes.
                            let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                            if full_checksum == self.state.checksum {
                                Ok(Either::Left(Ok(Self::next(Step::Footer, self.state))))
                            } else {
                                Ok(Either::Left(Err(ReceiveError::new(
                                    receive_error::Step::Footer,
                                    error::Receive::Checksum {
                                        calculated: self.state.checksum,
                                        received: full_checksum,
                                    },
                                    self.state.attempt,
                                ))))
                            }
                        } else {
                            self.state.checksum = self
                                .state
                                .checksum
                                .wrapping_add(bytes[2] as u16)
                                .wrapping_add(bytes[3] as u16);
                            if bytes_to_receive <= 4 {
                                // These are the last data bytes.
                                Ok(Either::Left(Ok(Self::next(Step::Checksum, self.state))))
                            } else {
                                // There is more data.
                                Ok(Either::Left(Ok(Self::next(
                                    Step::Data {
                                        length,
                                        index: index + 4,
                                    },
                                    self.state,
                                ))))
                            }
                        }
                    }
                    Step::Checksum => {
                        // The checksum is contained in the last two bytes.
                        self.state.checksum = self
                            .state
                            .checksum
                            .wrapping_add(bytes[0] as u16)
                            .wrapping_add(bytes[1] as u16);
                        let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                        if full_checksum == self.state.checksum {
                            Ok(Either::Left(Ok(Self::next(Step::Footer, self.state))))
                        } else {
                            Ok(Either::Left(Err(ReceiveError::new(
                                receive_error::Step::Footer,
                                error::Receive::Checksum {
                                    calculated: self.state.checksum,
                                    received: full_checksum,
                                },
                                self.state.attempt,
                            ))))
                        }
                    }
                    Step::Footer => {
                        match Adapter::try_from(bytes[0]) {
                            Ok(adapter) => match NonZeroU8::new(bytes[1]) {
                                None => {
                                    // We don't care about what the adapter was set to previously. We just
                                    // return the device this packet indicated.
                                    Ok(Either::Right(adapter))
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
    }
}
