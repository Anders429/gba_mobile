use super::{
    super::{MAX_RETRIES, Timeout, communication, error, schedule_serial},
    WaitForReceive,
};
use crate::{
    driver::{Command, frames},
    mmio::serial::{SIODATA8, TransferLength},
};
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(super) enum Step {
    HeaderCommand,
    HeaderEmptyByte,
    HeaderLength1,
    HeaderLength2 { first_byte: u8 },

    Data { index: u16, length: NonZeroU16 },

    Checksum1,
    Checksum2,

    FooterDevice,
    FooterCommand,
}

#[derive(Debug)]
struct State {
    error: error::Receive,
    attempt: u8,
    frame: u8,
    communication_state: communication::State,
}

impl State {
    fn new(error: error::Receive, attempt: u8) -> Self {
        Self {
            error,
            attempt,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }

    fn next(self) -> Self {
        Self {
            error: self.error,
            attempt: self.attempt,
            frame: self.frame,
            communication_state: communication::State::Send,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active) struct ReceiveError {
    step: Step,
    state: State,
}

impl ReceiveError {
    pub(super) fn new(step: Step, error: error::Receive, attempt: u8) -> Self {
        Self {
            step,
            state: State::new(error, attempt),
        }
    }

    fn next(step: Step, state: State) -> Self {
        Self {
            step,
            state: state.next(),
        }
    }
}

impl super::super::ReceiveError for ReceiveError {
    type WaitForReceive = WaitForReceive;

    fn vblank(&mut self) -> Result<(), Timeout> {
        if self.state.frame > frames::THREE_SECONDS {
            return Err(Timeout::Serial);
        } else {
            self.state.frame += 1;
            Ok(())
        }
    }

    fn timer(&mut self) {
        if matches!(self.state.communication_state, communication::State::Send) {
            let byte = match self.step {
                Step::FooterDevice { .. } => 0x81,
                Step::FooterCommand { .. } => {
                    if self.state.attempt + 1 < MAX_RETRIES {
                        self.state.error.command() as u8 | 0x80
                    } else {
                        // Since we've errored on communication too much, it doesn't matter what we
                        // send here. We are going to error out the entire link session anyway.
                        Command::Empty as u8 | 0x80
                    }
                }
                _ => 0x4b,
            };
            self.state.communication_state = communication::State::Receive;
            unsafe { SIODATA8.write_volatile(byte) };
            schedule_serial(TransferLength::_8Bit);
        }
    }

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Receive> {
        match self.state.communication_state {
            communication::State::Send => Ok(Either::Left(self)),
            communication::State::Receive => {
                let byte = unsafe { SIODATA8.read_volatile() };
                match self.step {
                    Step::HeaderCommand => {
                        Ok(Either::Left(Self::next(Step::HeaderEmptyByte, self.state)))
                    }
                    Step::HeaderEmptyByte => {
                        Ok(Either::Left(Self::next(Step::HeaderLength1, self.state)))
                    }
                    Step::HeaderLength1 => Ok(Either::Left(Self::next(
                        Step::HeaderLength2 { first_byte: byte },
                        self.state,
                    ))),
                    Step::HeaderLength2 { first_byte } => {
                        let full_length = ((first_byte as u16) << 8) | (byte as u16);
                        match NonZeroU16::new(full_length) {
                            Some(length) => Ok(Either::Left(Self::next(
                                Step::Data { index: 0, length },
                                self.state,
                            ))),
                            None => Ok(Either::Left(Self::next(Step::Checksum1, self.state))),
                        }
                    }
                    Step::Data { index, length } => {
                        if let Some(next_index) = index.checked_add(1)
                            && next_index < length.get()
                        {
                            Ok(Either::Left(Self::next(
                                Step::Data {
                                    index: next_index,
                                    length,
                                },
                                self.state,
                            )))
                        } else {
                            Ok(Either::Left(Self::next(Step::Checksum1, self.state)))
                        }
                    }
                    Step::Checksum1 => Ok(Either::Left(Self::next(Step::Checksum2, self.state))),
                    Step::Checksum2 => Ok(Either::Left(Self::next(Step::FooterDevice, self.state))),
                    Step::FooterDevice => {
                        Ok(Either::Left(Self::next(Step::FooterCommand, self.state)))
                    }
                    Step::FooterCommand => {
                        let new_attempt = self.state.attempt + 1;
                        if new_attempt < MAX_RETRIES {
                            // Retry.
                            Ok(Either::Right(WaitForReceive::new(new_attempt)))
                        } else {
                            // Too many retries. Stop trying and return error.
                            Err(self.state.error)
                        }
                    }
                }
            }
        }
    }
}
