mod accept;
mod connect;
mod end;
mod error;
mod idle;
mod request;
mod reset;
mod start;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{Phase, Queue, State, StateChange};
use crate::{
    ArrayVec, Generation, Timer, driver::Adapter, mmio::serial::TransferLength, phone_number::Digit,
};
use accept::Accept;
use connect::Connect;
use either::Either;
use end::End;
use idle::Idle;
use reset::Reset;
use start::Start;

#[derive(Debug)]
pub(super) enum Flow {
    Start(Start),
    End(End),
    Reset(Reset),

    Accept(Accept),
    Connect(Connect),

    Idle(Idle),
}

impl Flow {
    pub(super) fn start(transfer_length: TransferLength) -> Self {
        Self::Start(Start::new(transfer_length))
    }

    pub(super) fn end(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::End(End::new(transfer_length, timer))
    }

    pub(super) fn reset(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::Reset(Reset::new(transfer_length, timer))
    }

    pub(super) fn accept(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::Accept(Accept::new(transfer_length, timer))
    }

    pub(super) fn connect(
        transfer_length: TransferLength,
        timer: Timer,
        adapter: Adapter,
        phone_number: ArrayVec<Digit, 32>,
        connection_generation: Generation,
    ) -> Self {
        Self::Connect(Connect::new(
            transfer_length,
            timer,
            adapter,
            phone_number,
            connection_generation,
        ))
    }

    pub(super) fn idle(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::Idle(Idle::new(transfer_length, timer))
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::Start(start) => start.vblank().map(Self::Start).map_err(Timeout::Start),
            Self::End(end) => end.vblank().map(Self::End).map_err(Timeout::End),
            Self::Reset(reset) => reset.vblank().map(Self::Reset).map_err(Timeout::Reset),
            Self::Accept(accept) => accept.vblank().map(Self::Accept).map_err(Timeout::Accept),
            Self::Connect(connect) => connect
                .vblank()
                .map(Self::Connect)
                .map_err(Timeout::Connect),
            Self::Idle(idle) => idle.vblank().map(Self::Idle).map_err(Timeout::Idle),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::Start(start) => start.timer(),
            Self::End(end) => end.timer(),
            Self::Reset(reset) => reset.timer(),
            Self::Accept(accept) => accept.timer(),
            Self::Connect(connect) => connect.timer(),
            Self::Idle(idle) => idle.timer(),
        }
    }

    pub(super) fn serial(
        self,
        state: &mut State,
        queue: &mut Queue,
    ) -> Result<Either<Self, StateChange>, Error> {
        match self {
            Self::Start(start) => start
                .serial(
                    &mut state.adapter,
                    &mut state.transfer_length,
                    state.timer,
                    &mut state.phase,
                )
                .map(|response| match response {
                    Either::Left(start) => Either::Left(Self::Start(start)),
                    Either::Right(response) => {
                        match response {
                            start::Response::Success => {}
                            start::Response::AlreadyActive => {
                                queue.set_end();
                                queue.set_start();
                            }
                        }
                        Either::Right(StateChange::StillActive)
                    }
                })
                .map_err(Error::Start),
            Self::End(end) => end
                .serial(&mut state.adapter, &mut state.transfer_length, state.timer)
                .map(|flow| {
                    flow.map_or_else(
                        || {
                            if matches!(state.phase, Phase::Ending) {
                                Either::Right(StateChange::Inactive)
                            } else {
                                Either::Right(StateChange::StillActive)
                            }
                        },
                        |flow| Either::Left(Self::End(flow)),
                    )
                })
                .map_err(Error::End),
            Self::Reset(reset) => reset
                .serial(
                    &mut state.adapter,
                    &mut state.transfer_length,
                    state.timer,
                    &mut state.phase,
                )
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Reset(flow)),
                    )
                })
                .map_err(Error::Reset),
            Self::Accept(accept) => accept
                .serial(&mut state.adapter, &mut state.phase, state.timer)
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Accept(flow)),
                    )
                })
                .map_err(Error::Accept),
            Self::Connect(connect) => connect
                .serial(
                    &mut state.adapter,
                    &mut state.phase,
                    state.timer,
                    state.connection_generation,
                )
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Connect(flow)),
                    )
                })
                .map_err(Error::Connect),
            Self::Idle(idle) => idle
                .serial(&mut state.phase)
                .map(|_| Either::Right(StateChange::StillActive))
                .map_err(Error::Idle),
        }
    }
}
