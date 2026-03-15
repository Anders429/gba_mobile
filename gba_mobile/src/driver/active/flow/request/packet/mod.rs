pub(in crate::driver) mod error;
pub(in crate::driver::active::flow) mod payload;

mod sio32;
mod sio8;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{schedule_serial, schedule_timer};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength};
use either::Either;
use payload::Payload;
use sio8::Sio8;
use sio32::Sio32;

const MAX_RETRIES: u8 = 5;

trait Send: Sized {
    type WaitForReceive;

    fn vblank(self) -> Result<Self, Timeout>;

    fn timer(&mut self);

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Send>;
}

trait WaitForReceive: Sized {
    type Receive;
    type ReceiveError;

    fn vblank(self) -> Result<Self, Timeout>;

    fn serial(self) -> Result<Either<Self, Self::Receive>, Self::ReceiveError>;
}

trait Receive<Payload>: Sized
where
    Payload: self::Payload,
{
    type ReceiveError;

    fn vblank(self) -> Result<Self, Timeout>;

    fn timer(&self);

    fn serial(
        self,
    ) -> Result<Either<Result<Self, Self::ReceiveError>, Response<Payload>>, error::Receive<Payload>>;
}

trait ReceiveError<Payload>: Sized
where
    Payload: self::Payload,
{
    type WaitForReceive;

    fn vblank(self) -> Result<Self, Timeout>;

    fn timer(&self);

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Receive<Payload>>;
}

trait Sio {
    const TRANSFER_LENGTH: TransferLength;

    type Send<Payload>: Send<WaitForReceive = Self::WaitForReceive<Payload>>
    where
        Payload: self::Payload;
    type WaitForReceive<Payload>: WaitForReceive<Receive = Self::Receive<Payload>, ReceiveError = Self::ReceiveError<Payload>>
    where
        Payload: self::Payload;
    type Receive<Payload>: Receive<Payload, ReceiveError = Self::ReceiveError<Payload>>
    where
        Payload: self::Payload;
    type ReceiveError<Payload>: ReceiveError<Payload, WaitForReceive = Self::WaitForReceive<Payload>>
    where
        Payload: self::Payload;
}

#[derive(Debug)]
enum State<Payload, Sio>
where
    Payload: self::Payload,
    Sio: self::Sio,
{
    Send(Sio::Send<Payload>),
    WaitForReceive(Sio::WaitForReceive<Payload>),
    Receive(Sio::Receive<Payload>),
    ReceiveError(Sio::ReceiveError<Payload>),
}

impl<Payload, Sio> State<Payload, Sio>
where
    Payload: self::Payload,
    Sio: self::Sio,
{
    fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::Send(send) => Ok(Self::Send(send.vblank()?)),
            Self::WaitForReceive(wait_for_receive) => {
                Ok(Self::WaitForReceive(wait_for_receive.vblank()?))
            }
            Self::Receive(receive) => Ok(Self::Receive(receive.vblank()?)),
            Self::ReceiveError(receive_error) => Ok(Self::ReceiveError(receive_error.vblank()?)),
        }
    }

    fn timer(&mut self) {
        match self {
            Self::Send(send) => {
                send.timer();
                schedule_serial(Sio::TRANSFER_LENGTH);
            }
            Self::WaitForReceive(_) => {}
            Self::Receive(receive) => {
                receive.timer();
                schedule_serial(Sio::TRANSFER_LENGTH);
            }
            Self::ReceiveError(receive_error) => {
                receive_error.timer();
                schedule_serial(Sio::TRANSFER_LENGTH);
            }
        }
    }

    fn serial(self, timer: Timer) -> Result<Either<Self, Response<Payload>>, Error<Payload>> {
        match self {
            Self::Send(send) => Ok(Either::Left(
                send.serial()?
                    .map_left(|send| {
                        schedule_timer(timer, Sio::TRANSFER_LENGTH);
                        Self::Send(send)
                    })
                    .map_right(Self::WaitForReceive)
                    .into_inner(),
            )),
            Self::WaitForReceive(wait_for_receive) => Ok(Either::Left(
                Either::from(wait_for_receive.serial())
                    .map_right(|right| {
                        right
                            .map_left(Self::WaitForReceive)
                            .map_right(|receive| {
                                schedule_timer(timer, Sio::TRANSFER_LENGTH);
                                Self::Receive(receive)
                            })
                            .into_inner()
                    })
                    .map_left(|receive_error| {
                        schedule_timer(timer, Sio::TRANSFER_LENGTH);
                        Self::ReceiveError(receive_error)
                    })
                    .into_inner(),
            )),
            Self::Receive(receive) => {
                let result = Ok(receive.serial()?.map_left(|left| {
                    schedule_timer(timer, Sio::TRANSFER_LENGTH);
                    Either::from(left)
                        .map_right(Self::Receive)
                        .map_left(Self::ReceiveError)
                        .into_inner()
                }));
                result
            }
            Self::ReceiveError(receive_error) => Ok(Either::Left(
                receive_error
                    .serial()?
                    .map_left(|receive_error| {
                        schedule_timer(timer, Sio::TRANSFER_LENGTH);
                        Self::ReceiveError(receive_error)
                    })
                    .map_right(Self::WaitForReceive)
                    .into_inner(),
            )),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum Packet<Payload>
where
    Payload: self::Payload,
{
    Packet8(State<Payload, Sio8>),
    Packet32(State<Payload, Sio32>),
}

impl<Payload> Packet<Payload>
where
    Payload: self::Payload,
{
    pub(in crate::driver::active::flow) fn new(
        payload: Payload::Send,
        transfer_length: TransferLength,
        timer: Timer,
    ) -> Self {
        schedule_timer(timer, transfer_length);
        match transfer_length {
            TransferLength::_8Bit => Self::Packet8(State::Send(sio8::Send::new(payload))),
            TransferLength::_32Bit => Self::Packet32(State::Send(sio32::Send::new(payload))),
        }
    }

    pub(in crate::driver::active::flow) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::Packet8(packet) => packet.vblank().map(Self::Packet8),
            Self::Packet32(packet) => packet.vblank().map(Self::Packet32),
        }
    }

    pub(in crate::driver::active::flow) fn timer(&mut self) {
        match self {
            Self::Packet8(packet) => packet.timer(),
            Self::Packet32(packet) => packet.timer(),
        }
    }

    pub(in crate::driver::active::flow) fn serial(
        self,
        timer: Timer,
    ) -> Result<Either<Self, Response<Payload>>, Error<Payload>> {
        match self {
            Self::Packet8(packet) => packet.serial(timer).map(|ok| ok.map_left(Self::Packet8)),
            Self::Packet32(packet) => packet.serial(timer).map(|ok| ok.map_left(Self::Packet32)),
        }
    }
}

pub(in crate::driver::active::flow) struct Response<Payload>
where
    Payload: self::Payload,
{
    pub(in crate::driver::active::flow) payload: Payload::ReceiveParsed,
    pub(in crate::driver::active::flow) adapter: Adapter,
}
