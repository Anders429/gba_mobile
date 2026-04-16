use super::{
    super::{Payload, Timeout, communication, error, payload, schedule_serial},
    Receive, ReceiveError, receive_error,
};
use crate::{
    driver::{Command, active::flow::request::packet::payload::ReceiveCommand, frames},
    mmio::serial::{SIODATA32, TransferLength},
};
use either::Either;

#[derive(Debug)]
pub(in crate::driver::active) struct WaitForReceive<Payload>
where
    Payload: self::Payload,
{
    payload: Payload::ReceiveCommand,
    packet_frame: u16,
    serial_frame: u8,
    attempt: u8,
    communication_state: communication::State,
}

impl<Payload> WaitForReceive<Payload>
where
    Payload: self::Payload,
{
    pub(super) fn new(payload: Payload::ReceiveCommand, attempt: u8) -> Self {
        Self {
            payload,
            packet_frame: 0,
            serial_frame: 0,
            attempt,
            communication_state: communication::State::Send,
        }
    }

    fn reset(self) -> Self {
        Self {
            payload: self.payload,
            packet_frame: self.packet_frame,
            serial_frame: 0,
            attempt: self.attempt,
            communication_state: communication::State::Send,
        }
    }
}

impl<Payload> super::super::WaitForReceive for WaitForReceive<Payload>
where
    Payload: self::Payload,
{
    type Receive = Receive<Payload>;
    type ReceiveError = ReceiveError<Payload>;

    fn vblank(&mut self) -> Result<(), Timeout> {
        if self.packet_frame > frames::FIFTEEN_SECONDS {
            Err(Timeout::Packet)
        } else if self.serial_frame > frames::THREE_SECONDS {
            Err(Timeout::Serial)
        } else {
            if self.packet_frame % frames::ONE_HUNDRED_MILLISECONDS as u16 == 0
                && matches!(self.communication_state, communication::State::Send)
            {
                // Send new idle bytes every 100 milliseconds.
                unsafe { SIODATA32.write_volatile(0x4b4b4b4b) };
                self.communication_state = communication::State::Receive;
                schedule_serial(TransferLength::_32Bit);
            }
            self.packet_frame += 1;
            self.serial_frame += 1;
            Ok(())
        }
    }

    fn serial(self) -> Result<Either<Self, Self::Receive>, Self::ReceiveError> {
        match self.communication_state {
            communication::State::Send => Ok(Either::Left(self)),
            communication::State::Receive => {
                let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };

                match bytes[0] {
                    0xd2 => Ok(Either::Left(self.reset())),
                    0x99 => match bytes[1] {
                        0x66 => {
                            let command_xor = bytes[2] & 0x80 == 0;
                            match Command::try_from(bytes[2] & 0x7f) {
                                Ok(command) => match self.payload.receive_command(command) {
                                    Ok(payload) => Ok(Either::Right(Receive::new(
                                        payload,
                                        0,
                                        (bytes[2] as u16).wrapping_add(bytes[3] as u16),
                                        command_xor,
                                    ))),
                                    Err((error, payload)) => Err(ReceiveError::new(
                                        receive_error::Step::HeaderLength,
                                        payload,
                                        error::Receive::Payload(payload::Error::ReceiveCommand(
                                            error,
                                        )),
                                        self.attempt,
                                    )),
                                },
                                Err(unknown) => Err(ReceiveError::new(
                                    receive_error::Step::HeaderLength,
                                    self.payload,
                                    error::Receive::UnknownCommand(unknown),
                                    self.attempt,
                                )),
                            }
                        }
                        byte => Err(ReceiveError::new(
                            receive_error::Step::HeaderLength,
                            self.payload,
                            error::Receive::MagicValue2(byte),
                            self.attempt,
                        )),
                    },
                    byte => Err(ReceiveError::new(
                        receive_error::Step::HeaderLength,
                        self.payload,
                        error::Receive::MagicValue1(byte),
                        self.attempt,
                    )),
                }
            }
        }
    }
}
