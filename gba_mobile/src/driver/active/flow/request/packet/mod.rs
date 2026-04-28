pub(in crate::driver) mod error;

pub(in crate::driver::active::flow) mod payload;

mod data;
mod sio32;
mod sio8;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

pub(in crate::driver::active) use data::Data;

pub(in crate::driver::active::flow) use payload::Payload;

use super::{communication, schedule_serial, schedule_timer};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength};
use either::Either;
use sio8::Sio8;
use sio32::Sio32;

const MAX_RETRIES: u8 = 5;

trait Send: Sized {
    type WaitForReceive;

    fn vblank(&mut self) -> Result<(), Timeout>;

    fn timer(&mut self, data: &Data);

    fn serial(self, data: &Data) -> Result<Either<Self, Self::WaitForReceive>, error::Send>;
}

trait WaitForReceive: Sized {
    type Receive;
    type ReceiveError;

    fn vblank(&mut self) -> Result<(), Timeout>;

    fn serial(self, data: &mut Data) -> Result<Either<Self, Self::Receive>, Self::ReceiveError>;
}

trait Receive: Sized {
    type ReceiveError;

    fn vblank(&mut self) -> Result<(), Timeout>;

    fn timer(&mut self, data: &Data);

    fn serial(
        self,
        data: &mut Data,
    ) -> Result<Either<Result<Self, Self::ReceiveError>, Adapter>, error::Receive>;
}

trait ReceiveError: Sized {
    type WaitForReceive;

    fn vblank(&mut self) -> Result<(), Timeout>;

    fn timer(&mut self);

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Receive>;
}

trait Sio {
    const TRANSFER_LENGTH: TransferLength;

    type Send: Send<WaitForReceive = Self::WaitForReceive>;
    type WaitForReceive: WaitForReceive<Receive = Self::Receive, ReceiveError = Self::ReceiveError>;
    type Receive: Receive<ReceiveError = Self::ReceiveError>;
    type ReceiveError: ReceiveError<WaitForReceive = Self::WaitForReceive>;
}

#[derive(Debug)]
enum Operation<Sio>
where
    Sio: self::Sio,
{
    Send(Sio::Send),
    WaitForReceive(Sio::WaitForReceive),
    Receive(Sio::Receive),
    ReceiveError(Sio::ReceiveError),
}

impl<Sio> Operation<Sio>
where
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

    fn timer(&mut self, data: &Data) {
        match self {
            Self::Send(send) => send.timer(data),
            Self::WaitForReceive(_) => {}
            Self::Receive(receive) => receive.timer(data),
            Self::ReceiveError(receive_error) => receive_error.timer(),
        }
    }

    fn serial<Payload>(
        self,
        timer: Timer,
        data: &mut Data,
    ) -> Result<Either<Self, Adapter>, Error<Payload>>
    where
        Payload: self::Payload,
    {
        match self {
            Self::Send(send) => Ok(Either::Left(
                send.serial(data)?
                    .map_left(Self::Send)
                    .map_right(Self::WaitForReceive)
                    .into_inner(),
            )),
            Self::WaitForReceive(wait_for_receive) => Ok(Either::Left(
                Either::from(wait_for_receive.serial(data))
                    .map_right(|right| {
                        right
                            .map_left(Self::WaitForReceive)
                            .map_right(Self::Receive)
                            .into_inner()
                    })
                    .map_left(Self::ReceiveError)
                    .into_inner(),
            )),
            Self::Receive(receive) => Ok(receive.serial(data)?.map_left(|left| {
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
enum State {
    Packet8(Operation<Sio8>),
    Packet32(Operation<Sio32>),
}

impl State {
    fn new(transfer_length: TransferLength, timer: Timer) -> Self {
        schedule_timer(timer, transfer_length);
        match transfer_length {
            TransferLength::_8Bit => Self::Packet8(Operation::Send(sio8::Send::new())),
            TransferLength::_32Bit => Self::Packet32(Operation::Send(sio32::Send::new())),
        }
    }

    fn vblank(&mut self) -> Result<(), Timeout> {
        match self {
            Self::Packet8(packet) => packet.vblank(),
            Self::Packet32(packet) => packet.vblank(),
        }
    }

    fn timer(&mut self, data: &Data) {
        match self {
            Self::Packet8(packet) => packet.timer(data),
            Self::Packet32(packet) => packet.timer(data),
        }
    }

    fn serial<Payload>(
        self,
        timer: Timer,
        data: &mut Data,
    ) -> Result<Either<Self, Adapter>, Error<Payload>>
    where
        Payload: self::Payload,
    {
        match self {
            Self::Packet8(packet) => packet
                .serial(timer, data)
                .map(|ok| ok.map_left(Self::Packet8)),
            Self::Packet32(packet) => packet
                .serial(timer, data)
                .map(|ok| ok.map_left(Self::Packet32)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Packet<Payload> {
    state: State,
    payload: Payload,
}

impl<Payload> Packet<Payload>
where
    Payload: self::Payload,
{
    pub(in crate::driver::active::flow) fn new(
        payload: Payload,
        transfer_length: TransferLength,
        timer: Timer,
    ) -> Self {
        Self {
            state: State::new(transfer_length, timer),
            payload,
        }
    }

    pub(in crate::driver::active::flow) fn vblank(&mut self) -> Result<(), Timeout> {
        self.state.vblank()
    }

    pub(in crate::driver::active::flow) fn timer(&mut self, data: &Data) {
        self.state.timer(data)
    }

    pub(in crate::driver::active::flow) fn serial<'a, 'b>(
        self,
        timer: Timer,
        data: &'a mut Data,
    ) -> Result<Either<Self, Response<'b, Payload>>, Error<Payload>>
    where
        'a: 'b,
    {
        self.state
            .serial(timer, data)
            .and_then(|either| match either {
                Either::Left(state) => Ok(Either::Left(Self {
                    state,
                    payload: self.payload,
                })),
                Either::Right(adapter) => self
                    .payload
                    .parse(data)
                    .map(|response| {
                        Either::Right(Response {
                            payload: response,
                            adapter,
                        })
                    })
                    .map_err(Error::Payload),
            })
    }
}

pub(in crate::driver::active::flow) struct Response<'a, Payload>
where
    Payload: self::Payload,
{
    pub(in crate::driver::active::flow) payload: Payload::Response<'a>,
    pub(in crate::driver::active::flow) adapter: Adapter,
}
