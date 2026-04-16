pub(in crate::driver) mod error;
pub(in crate::driver::active::flow) mod payload;

mod sio32;
mod sio8;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{communication, schedule_serial, schedule_timer};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength};
use either::Either;
use payload::Payload;
use sio8::Sio8;
use sio32::Sio32;

const MAX_RETRIES: u8 = 5;

pub(in crate::driver::active) trait Send: Sized {
    type WaitForReceive;

    fn vblank(&mut self) -> Result<(), Timeout>;

    fn timer(&mut self);

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Send>;
}

pub(in crate::driver::active) trait WaitForReceive: Sized {
    type Receive;
    type ReceiveError;

    fn vblank(&mut self) -> Result<(), Timeout>;

    fn serial(self) -> Result<Either<Self, Self::Receive>, Self::ReceiveError>;
}

pub(in crate::driver::active) trait Receive<Payload>: Sized
where
    Payload: self::Payload,
{
    type ReceiveError;

    fn vblank(&mut self) -> Result<(), Timeout>;

    fn timer(&mut self);

    fn serial(
        self,
    ) -> Result<Either<Result<Self, Self::ReceiveError>, Response<Payload>>, error::Receive<Payload>>;
}

pub(in crate::driver::active) trait ReceiveError<Payload>: Sized
where
    Payload: self::Payload,
{
    type WaitForReceive;

    fn vblank(&mut self) -> Result<(), Timeout>;

    fn timer(&mut self);

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Receive<Payload>>;
}

pub(in crate::driver::active) trait Sio {
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
pub(in crate::driver::active) enum State<Payload, Sio>
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
    fn vblank(&mut self) -> Result<(), Timeout> {
        match self {
            Self::Send(send) => send.vblank(),
            Self::WaitForReceive(wait_for_receive) => wait_for_receive.vblank(),
            Self::Receive(receive) => receive.vblank(),
            Self::ReceiveError(receive_error) => receive_error.vblank(),
        }
    }

    fn timer(&mut self) {
        match self {
            Self::Send(send) => send.timer(),
            Self::WaitForReceive(_) => {}
            Self::Receive(receive) => receive.timer(),
            Self::ReceiveError(receive_error) => receive_error.timer(),
        }
    }

    fn serial(self, timer: Timer) -> Result<Either<Self, Response<Payload>>, Error<Payload>> {
        match self {
            Self::Send(send) => Ok(Either::Left(
                send.serial()?
                    .map_left(Self::Send)
                    .map_right(Self::WaitForReceive)
                    .into_inner(),
            )),
            Self::WaitForReceive(wait_for_receive) => Ok(Either::Left(
                Either::from(wait_for_receive.serial())
                    .map_right(|right| {
                        right
                            .map_left(Self::WaitForReceive)
                            .map_right(Self::Receive)
                            .into_inner()
                    })
                    .map_left(Self::ReceiveError)
                    .into_inner(),
            )),
            Self::Receive(receive) => Ok(receive.serial()?.map_left(|left| {
                Either::from(left)
                    .map_right(Self::Receive)
                    .map_left(Self::ReceiveError)
                    .into_inner()
            })),
            Self::ReceiveError(receive_error) => Ok(Either::Left(
                receive_error
                    .serial()?
                    .map_left(Self::ReceiveError)
                    .map_right(Self::WaitForReceive)
                    .into_inner(),
            )),
        }
        .map(|response| {
            response.map_left(|state| {
                // No matter whether we actually received data here, we still want to make sure the
                // timer is running.
                state.schedule_timer(timer);
                state
            })
        })
    }

    fn schedule_timer(&self, timer: Timer) {
        match self {
            Self::Send(_) => schedule_timer(timer, Sio::TRANSFER_LENGTH),
            Self::WaitForReceive(_) => {}
            Self::Receive(_) => schedule_timer(timer, Sio::TRANSFER_LENGTH),
            Self::ReceiveError(_) => schedule_timer(timer, Sio::TRANSFER_LENGTH),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active) enum Packet<Payload>
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

    pub(in crate::driver::active::flow) fn vblank(&mut self) -> Result<(), Timeout> {
        match self {
            Self::Packet8(packet) => packet.vblank(),
            Self::Packet32(packet) => packet.vblank(),
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
