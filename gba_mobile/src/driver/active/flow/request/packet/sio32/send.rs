use super::{
    super::{
        MAX_RETRIES, Payload, Timeout, communication, error, payload::Send as _, schedule_serial,
    },
    WaitForReceive,
};
use crate::{
    driver::{Command, frames},
    mmio::serial::{SIODATA32, TransferLength},
};
use either::Either;

#[derive(Debug)]
enum Step {
    MagicByte,
    HeaderLength,
    Data { index: u8 },
    Checksum,
    Footer,
}

#[derive(Debug)]
pub(in crate::driver::active) struct Send<Payload>
where
    Payload: self::Payload,
{
    payload: Payload::Send,
    step: Step,
    checksum: u16,
    attempt: u8,
    frame: u8,
    communication_state: communication::State,
}

impl<Payload> Send<Payload>
where
    Payload: self::Payload,
{
    pub(in super::super) fn new(payload: Payload::Send) -> Self {
        Self {
            payload,
            step: Step::MagicByte,
            checksum: 0,
            attempt: 0,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }

    /// Advance to the given step.
    fn next(self, step: Step) -> Self {
        Self {
            payload: self.payload,
            step,
            checksum: self.checksum,
            attempt: self.attempt,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }

    /// Re-attempt this packet.
    fn retry(self, new_attempt: u8) -> Self {
        Self {
            payload: self.payload,
            step: Step::MagicByte,
            checksum: 0,
            attempt: new_attempt,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }
}

impl<Payload> super::super::Send for Send<Payload>
where
    Payload: self::Payload,
{
    type WaitForReceive = WaitForReceive<Payload>;

    fn vblank(&mut self) -> Result<(), Timeout> {
        if self.frame > frames::THREE_SECONDS {
            return Err(Timeout::Serial);
        } else {
            self.frame += 1;
            Ok(())
        }
    }

    fn timer(&mut self) {
        if matches!(self.communication_state, communication::State::Send) {
            let bytes = match self.step {
                Step::MagicByte => {
                    let command = self.payload.command() as u8;
                    self.checksum = self.checksum.wrapping_add(command as u16);
                    u32::from_be_bytes([0x99, 0x66, command, 0x00])
                }
                Step::HeaderLength => {
                    let length = self.payload.length();
                    self.checksum = self.checksum.wrapping_add(length as u16);
                    if length == 0 {
                        // If not sending any data, we skip straight to sending the checksum.
                        u32::from_be_bytes([
                            0x00,
                            length,
                            (self.checksum >> 8) as u8,
                            self.checksum as u8,
                        ])
                    } else {
                        let data_0 = self.payload.get(0);
                        let data_1 = self.payload.get(1);
                        self.checksum = self
                            .checksum
                            .wrapping_add(data_0 as u16)
                            .wrapping_add(data_1 as u16);
                        u32::from_be_bytes([0x00, length, data_0, data_1])
                    }
                }
                Step::Data { index } => {
                    let length = self.payload.length();
                    let mut bytes = [0x00; 4];
                    let mut offset = 0;
                    while offset < 4 && index + offset < length {
                        let byte = self.payload.get(index + offset);
                        self.checksum = self.checksum.wrapping_add(byte as u16);
                        bytes[offset as usize] = byte;
                        offset += 1;
                    }
                    if offset < 3 {
                        // If we have room, we pack the checksum in as well.
                        bytes[2] = (self.checksum >> 8) as u8;
                        bytes[3] = self.checksum as u8;
                    }
                    u32::from_be_bytes(bytes)
                }
                Step::Checksum => u32::from_be_bytes([
                    0x00,
                    0x00,
                    (self.checksum >> 8) as u8,
                    self.checksum as u8,
                ]),
                Step::Footer => 0x81_00_00_00,
            };

            self.communication_state = communication::State::Receive;
            unsafe { SIODATA32.write_volatile(bytes) };
            schedule_serial(TransferLength::_32Bit);
        }
    }

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Send> {
        match self.communication_state {
            communication::State::Send => Ok(Either::Left(self)),
            communication::State::Receive => {
                let bytes = unsafe { SIODATA32.read_volatile() };
                match self.step {
                    Step::MagicByte => Ok(Either::Left(self.next(Step::HeaderLength))),
                    Step::HeaderLength => {
                        let next_step = match self.payload.length() {
                            0 => Step::Footer,
                            1..=2 => Step::Checksum,
                            _ => Step::Data { index: 2 },
                        };
                        Ok(Either::Left(self.next(next_step)))
                    }
                    Step::Data { index } => {
                        let length = self.payload.length();
                        let next_step = if index + 2 >= length {
                            // We can fit the checksum in here.
                            Step::Footer
                        } else if index + 4 >= length {
                            Step::Checksum
                        } else {
                            Step::Data { index: index + 4 }
                        };
                        Ok(Either::Left(self.next(next_step)))
                    }
                    Step::Checksum => Ok(Either::Left(self.next(Step::Footer))),
                    Step::Footer => {
                        let new_attempt = self.attempt + 1;
                        match Command::try_from(bytes.to_be_bytes()[1] ^ 0x80) {
                            Ok(
                                Command::NotSupportedError
                                | Command::MalformedError
                                | Command::InternalError,
                            ) if new_attempt < MAX_RETRIES => {
                                // Retry.
                                Ok(Either::Left(self.retry(new_attempt)))
                            }
                            Ok(Command::NotSupportedError) => {
                                // Too many retries. Stop trying and return error.
                                Err(error::Send::UnsupportedCommand(self.payload.command()))
                            }
                            Ok(Command::MalformedError) => {
                                // Too many retries. Stop trying and return error.
                                Err(error::Send::Malformed)
                            }
                            Ok(Command::InternalError) => {
                                // Too many retries. Stop trying and return error.
                                Err(error::Send::AdapterInternalError)
                            }
                            _ => {
                                // We don't verify anything here and simply assume the adapter responded
                                // with a correct command. If the adapter is in an invalid state, we will
                                // find out when receiving the response packet.
                                Ok(Either::Right(WaitForReceive::new(self.payload.finish(), 0)))
                            }
                        }
                    }
                }
            }
        }
    }
}
