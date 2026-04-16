use super::{
    super::{Payload, Timeout, communication, error, schedule_serial},
    Receive, ReceiveError, receive_error,
};
use crate::{
    driver::frames,
    mmio::serial::{SIODATA8, TransferLength},
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
                // Send a new idle byte every 100 milliseconds.
                unsafe { SIODATA8.write_volatile(0x4b) };
                self.communication_state = communication::State::Receive;
                schedule_serial(TransferLength::_8Bit);
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
                let byte = unsafe { SIODATA8.read_volatile() };

                match byte {
                    0x99 => Ok(Either::Right(Receive::new(self.payload, self.attempt))),
                    0xd2 => Ok(Either::Left(self.reset())),
                    // Anything else is not proper communication and should enter an error state.
                    _ => Err(ReceiveError::new(
                        receive_error::Step::MagicByte2,
                        self.payload,
                        error::Receive::MagicValue1(byte),
                        self.attempt,
                    )),
                }
            }
        }
    }
}
