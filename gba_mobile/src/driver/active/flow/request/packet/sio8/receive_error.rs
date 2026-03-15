use super::{
    super::{MAX_RETRIES, Payload, Timeout, error},
    WaitForReceive,
};
use crate::{
    driver::{Command, frames},
    mmio::serial::SIODATA8,
};
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(super) enum Step {
    MagicByte2,

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
struct State<Payload>
where
    Payload: self::Payload,
{
    payload: Payload::ReceiveCommand,
    error: error::Receive<Payload>,
    attempt: u8,
    frame: u8,
}

impl<Payload> State<Payload>
where
    Payload: self::Payload,
{
    fn new(payload: Payload::ReceiveCommand, error: error::Receive<Payload>, attempt: u8) -> Self {
        Self {
            payload,
            error,
            attempt,
            frame: 0,
        }
    }

    fn next(self) -> Self {
        Self {
            payload: self.payload,
            error: self.error,
            attempt: self.attempt,
            frame: self.frame,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active) struct ReceiveError<Payload>
where
    Payload: self::Payload,
{
    step: Step,
    state: State<Payload>,
}

impl<Payload> ReceiveError<Payload>
where
    Payload: self::Payload,
{
    pub(super) fn new(
        step: Step,
        payload: Payload::ReceiveCommand,
        error: error::Receive<Payload>,
        attempt: u8,
    ) -> Self {
        Self {
            step,
            state: State::new(payload, error, attempt),
        }
    }

    fn next(step: Step, state: State<Payload>) -> Self {
        Self {
            step,
            state: state.next(),
        }
    }
}

impl<Payload> super::super::ReceiveError<Payload> for ReceiveError<Payload>
where
    Payload: self::Payload,
{
    type WaitForReceive = WaitForReceive<Payload>;

    fn vblank(mut self) -> Result<Self, Timeout> {
        if self.state.frame > frames::THREE_SECONDS {
            return Err(Timeout::Serial);
        } else {
            self.state.frame += 1;
            Ok(self)
        }
    }

    fn timer(&self) {
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
        unsafe { SIODATA8.write_volatile(byte) };
    }

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Receive<Payload>> {
        let byte = unsafe { SIODATA8.read_volatile() };
        log::debug!("received error byte: {byte:#04x}");
        match self.step {
            Step::MagicByte2 => Ok(Either::Left(Self::next(Step::HeaderCommand, self.state))),
            Step::HeaderCommand => Ok(Either::Left(Self::next(Step::HeaderEmptyByte, self.state))),
            Step::HeaderEmptyByte => Ok(Either::Left(Self::next(Step::HeaderLength1, self.state))),
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
            Step::FooterDevice => Ok(Either::Left(Self::next(Step::FooterCommand, self.state))),
            Step::FooterCommand => {
                let new_attempt = self.state.attempt + 1;
                if new_attempt < MAX_RETRIES {
                    // Retry.
                    Ok(Either::Right(WaitForReceive::new(
                        self.state.payload,
                        new_attempt,
                    )))
                } else {
                    // Too many retries. Stop trying and return error.
                    Err(self.state.error)
                }
            }
        }
    }
}
