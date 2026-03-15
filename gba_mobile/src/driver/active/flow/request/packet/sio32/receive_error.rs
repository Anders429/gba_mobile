use super::{
    super::{MAX_RETRIES, Payload, Timeout, error},
    WaitForReceive,
};
use crate::{
    driver::{Command, frames},
    mmio::serial::SIODATA32,
};
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(super) enum Step {
    HeaderLength,
    Data { index: u16, length: NonZeroU16 },
    Checksum,
    Footer,
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
pub(in super::super) struct ReceiveError<Payload>
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
        let bytes = match self.step {
            Step::Footer => {
                let command_byte = if self.state.attempt + 1 < MAX_RETRIES {
                    self.state.error.command() as u8 | 0x80
                } else {
                    // Since we've errored too many times, it doesn't matter what we send here. We
                    // will be propagating the error up through the driver anyway. Sending an empty
                    // command instead of an error command means the adapter won't try to send us
                    // another packet.
                    Command::Empty as u8 | 0x80
                };
                u32::from_be_bytes([0x81, command_byte, 0x00, 0x00])
            }
            _ => 0x4b_4b_4b_4b,
        };
        unsafe { SIODATA32.write_volatile(bytes) };
    }

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Receive<Payload>> {
        let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };
        match self.step {
            Step::HeaderLength => {
                let full_length = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                match NonZeroU16::new(full_length) {
                    Some(length) => {
                        if 2 < length.get() {
                            Ok(Either::Left(Self::next(Step::Checksum, self.state)))
                        } else {
                            Ok(Either::Left(Self::next(
                                Step::Data { length, index: 2 },
                                self.state,
                            )))
                        }
                    }
                    None => Ok(Either::Left(Self::next(Step::Footer, self.state))),
                }
            }
            Step::Data { index, length } => {
                if index + 2 >= length.get() {
                    // Checksum is included in last two bytes.
                    Ok(Either::Left(Self::next(Step::Footer, self.state)))
                } else if index + 4 >= length.get() {
                    // These are the last data bytes.
                    Ok(Either::Left(Self::next(Step::Checksum, self.state)))
                } else {
                    // There is more data.
                    Ok(Either::Left(Self::next(
                        Step::Data {
                            length,
                            index: index + 4,
                        },
                        self.state,
                    )))
                }
            }
            Step::Checksum => Ok(Either::Left(Self::next(Step::Footer, self.state))),
            Step::Footer => {
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
