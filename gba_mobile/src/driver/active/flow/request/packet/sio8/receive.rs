use core::num::{NonZeroU8, NonZeroU16};

use super::{
    super::{Data, Timeout, communication, error, schedule_serial},
    ReceiveError, receive_error,
};
use crate::{
    driver::{Adapter, Command, frames},
    mmio::serial::{SIODATA8, TransferLength},
};
use either::Either;

#[derive(Debug)]
enum Step {
    MagicByte2,

    HeaderCommand,
    HeaderEmptyByte,
    HeaderLength1,
    HeaderLength2 { first_byte: u8 },

    Data { index: u8, length: NonZeroU8 },

    Checksum1,
    Checksum2 { first_byte: u8 },

    FooterDevice,
    FooterCommand { adapter: Adapter },
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
    fn new(attempt: u8) -> Self {
        Self {
            command_xor: false,
            checksum: 0,
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
    pub(super) fn new(attempt: u8) -> Self {
        Self {
            step: Step::MagicByte2,
            state: State::new(attempt),
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
            let byte = match &self.step {
                Step::FooterDevice => 0x81,
                Step::FooterCommand { .. } => {
                    if self.state.command_xor {
                        data.command as u8 | 0x80
                    } else {
                        data.command as u8
                    }
                }
                _ => 0x4b,
            };
            self.state.communication_state = communication::State::Receive;
            unsafe { SIODATA8.write_volatile(byte) };
            schedule_serial(TransferLength::_8Bit);
        }
    }

    fn serial(
        mut self,
        data: &mut Data,
    ) -> Result<Either<Result<Self, Self::ReceiveError>, Adapter>, error::Receive> {
        match self.state.communication_state {
            communication::State::Send => Ok(Either::Left(Ok(self))),
            communication::State::Receive => {
                let byte = unsafe { SIODATA8.read_volatile() };
                match self.step {
                    Step::MagicByte2 => match byte {
                        0x66 => Ok(Either::Left(Ok(Self::next(
                            Step::HeaderCommand,
                            self.state,
                        )))),
                        _ => Ok(Either::Left(Err(ReceiveError::new(
                            receive_error::Step::HeaderCommand,
                            error::Receive::MagicValue2(byte),
                            self.state.attempt,
                        )))),
                    },
                    Step::HeaderCommand => {
                        self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                        self.state.command_xor = byte & 0x80 == 0;
                        match Command::try_from(byte & 0x7f) {
                            Ok(command) => {
                                data.command = command;
                                Ok(Either::Left(Ok(Self::next(
                                    Step::HeaderEmptyByte,
                                    self.state,
                                ))))
                            }
                            Err(unknown) => Ok(Either::Left(Err(ReceiveError::new(
                                receive_error::Step::HeaderEmptyByte,
                                error::Receive::UnknownCommand(unknown),
                                self.state.attempt,
                            )))),
                        }
                    }
                    Step::HeaderEmptyByte => {
                        self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                        Ok(Either::Left(Ok(Self::next(
                            Step::HeaderLength1,
                            self.state,
                        ))))
                    }
                    Step::HeaderLength1 => {
                        self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                        Ok(Either::Left(Ok(Self::next(
                            Step::HeaderLength2 { first_byte: byte },
                            self.state,
                        ))))
                    }
                    Step::HeaderLength2 { first_byte } => {
                        self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                        if first_byte > 0 {
                            let full_length = ((first_byte as u16) << 8) | (byte as u16);
                            Ok(Either::Left(Err(ReceiveError::new(
                                receive_error::Step::Data {
                                    index: 0,
                                    length: unsafe { NonZeroU16::new_unchecked(full_length) },
                                },
                                error::Receive::LengthTooLarge(full_length),
                                self.state.attempt,
                            ))))
                        } else if let Some(nonzero_length) = NonZeroU8::new(byte) {
                            Ok(Either::Left(Ok(Self::next(
                                Step::Data {
                                    index: 0,
                                    length: nonzero_length,
                                },
                                self.state,
                            ))))
                        } else {
                            Ok(Either::Left(Ok(Self::next(Step::Checksum1, self.state))))
                        }
                    }
                    Step::Data { index, length } => {
                        self.state.checksum = self.state.checksum.wrapping_add(byte as u16);
                        unsafe {
                            data.data.try_push(byte).unwrap_unchecked();
                        }
                        if let Some(next_index) = index.checked_add(1)
                            && next_index < length.get()
                        {
                            Ok(Either::Left(Ok(Self::next(
                                Step::Data {
                                    index: next_index,
                                    length,
                                },
                                self.state,
                            ))))
                        } else {
                            Ok(Either::Left(Ok(Self::next(Step::Checksum1, self.state))))
                        }
                    }
                    Step::Checksum1 => Ok(Either::Left(Ok(Self::next(
                        Step::Checksum2 { first_byte: byte },
                        self.state,
                    )))),
                    Step::Checksum2 { first_byte } => {
                        let full_checksum = ((first_byte as u16) << 8) | (byte as u16);
                        if full_checksum == self.state.checksum {
                            Ok(Either::Left(Ok(Self::next(Step::FooterDevice, self.state))))
                        } else {
                            Ok(Either::Left(Err(ReceiveError::new(
                                receive_error::Step::FooterDevice,
                                error::Receive::Checksum {
                                    calculated: self.state.checksum,
                                    received: full_checksum,
                                },
                                self.state.attempt,
                            ))))
                        }
                    }
                    Step::FooterDevice => match Adapter::try_from(byte) {
                        Ok(adapter) => Ok(Either::Left(Ok(Self::next(
                            Step::FooterCommand { adapter },
                            self.state,
                        )))),
                        Err(unknown) => Ok(Either::Left(Err(ReceiveError::new(
                            receive_error::Step::FooterCommand,
                            error::Receive::UnsupportedDevice(unknown),
                            self.state.attempt,
                        )))),
                    },
                    Step::FooterCommand { adapter } => {
                        // The acknowledgement signal command we receive is expected to be 0x00.
                        if let Some(nonzero) = NonZeroU8::new(byte) {
                            // We can no longer retry at this point. We simply enter an error state.
                            Err(error::Receive::NonZeroFooterCommand(nonzero))
                        } else {
                            // We don't care about what the adapter was set to previously. We just
                            // return the device this packet indicated.
                            Ok(Either::Right(adapter))
                        }
                    }
                }
            }
        }
    }
}
