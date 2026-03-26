mod accept;
mod connect;
mod end;
mod error;
mod idle;
mod login;
mod open_tcp;
mod open_udp;
mod request;
mod reset;
mod start;
mod status;
mod timeout;
mod write_config;

use core::net::{Ipv4Addr, SocketAddrV4};

use deranged::RangedU8;
pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{Phase, Queue, State, StateChange};
use crate::{ArrayVec, Digit, Generation, Timer, driver::Adapter, mmio::serial::TransferLength};
use accept::Accept;
use connect::Connect;
use either::Either;
use end::End;
use idle::Idle;
use login::Login;
use open_tcp::OpenTcp;
use open_udp::OpenUdp;
use reset::Reset;
use start::Start;
use status::Status;
use write_config::WriteConfig;

#[derive(Debug)]
pub(super) enum Flow {
    Start(Start),
    End(End),
    Reset(Reset),

    Accept(Accept),
    Connect(Connect),
    Login(Login),

    OpenTcp(OpenTcp),
    OpenUdp(OpenUdp),

    WriteConfig(WriteConfig),

    Status(Status),
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

    pub(super) fn login(
        transfer_length: TransferLength,
        timer: Timer,
        adapter: Adapter,
        phone_number: ArrayVec<Digit, 32>,
        id: ArrayVec<u8, 32>,
        password: ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
        connection_generation: Generation,
    ) -> Self {
        Self::Login(Login::new(
            transfer_length,
            timer,
            adapter,
            phone_number,
            id,
            password,
            primary_dns,
            secondary_dns,
            connection_generation,
        ))
    }

    pub(super) fn open_tcp_with_dns(
        transfer_length: TransferLength,
        timer: Timer,
        domain: ArrayVec<u8, 255>,
        port: u16,
        socket_index: RangedU8<0, 1>,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::OpenTcp(OpenTcp::with_dns(
            transfer_length,
            timer,
            domain,
            port,
            socket_index,
            connection_generation,
            socket_generation,
        ))
    }

    pub(super) fn open_tcp_with_socket_addr(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        socket_index: RangedU8<0, 1>,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::OpenTcp(OpenTcp::with_socket_addr(
            transfer_length,
            timer,
            addr,
            socket_index,
            connection_generation,
            socket_generation,
        ))
    }

    pub(super) fn open_udp_with_dns(
        transfer_length: TransferLength,
        timer: Timer,
        domain: ArrayVec<u8, 255>,
        port: u16,
        socket_index: RangedU8<0, 1>,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::OpenUdp(OpenUdp::with_dns(
            transfer_length,
            timer,
            domain,
            port,
            socket_index,
            connection_generation,
            socket_generation,
        ))
    }

    pub(super) fn open_udp_with_socket_addr(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        socket_index: RangedU8<0, 1>,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::OpenUdp(OpenUdp::with_socket_addr(
            transfer_length,
            timer,
            addr,
            socket_index,
            connection_generation,
            socket_generation,
        ))
    }

    pub(super) fn write_config(
        transfer_length: TransferLength,
        timer: Timer,
        config: &[u8; 256],
    ) -> Self {
        Self::WriteConfig(WriteConfig::new(transfer_length, timer, config))
    }

    pub(super) fn status(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::Status(Status::new(transfer_length, timer))
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
            Self::Login(login) => login.vblank().map(Self::Login).map_err(Timeout::Login),
            Self::OpenTcp(open_tcp) => open_tcp
                .vblank()
                .map(Self::OpenTcp)
                .map_err(Timeout::OpenTcp),
            Self::OpenUdp(open_udp) => open_udp
                .vblank()
                .map(Self::OpenUdp)
                .map_err(Timeout::OpenUdp),
            Self::WriteConfig(write_config) => write_config
                .vblank()
                .map(Self::WriteConfig)
                .map_err(Timeout::WriteConfig),
            Self::Status(status) => status.vblank().map(Self::Status).map_err(Timeout::Status),
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
            Self::Login(login) => login.timer(),
            Self::OpenTcp(open_tcp) => open_tcp.timer(),
            Self::OpenUdp(open_udp) => open_udp.timer(),
            Self::WriteConfig(write_config) => write_config.timer(),
            Self::Status(status) => status.timer(),
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
                    state.timer,
                    &mut state.adapter,
                    &mut state.transfer_length,
                    &mut state.phase,
                    &mut state.config,
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
                .serial(state.timer, &mut state.adapter, &mut state.transfer_length)
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
                    state.timer,
                    &mut state.adapter,
                    &mut state.transfer_length,
                    &mut state.phase,
                    &mut state.config,
                )
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Reset(flow)),
                    )
                })
                .map_err(Error::Reset),
            Self::Accept(accept) => accept
                .serial(
                    state.timer,
                    &mut state.adapter,
                    &mut state.phase,
                    &mut state.sockets[0],
                )
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Accept(flow)),
                    )
                })
                .map_err(Error::Accept),
            Self::Connect(connect) => connect
                .serial(
                    state.timer,
                    &mut state.adapter,
                    &mut state.phase,
                    &mut state.sockets[0],
                    state.connection_generation,
                )
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Connect(flow)),
                    )
                })
                .map_err(Error::Connect),
            Self::Login(login) => login
                .serial(
                    state.timer,
                    &mut state.adapter,
                    state.transfer_length,
                    &mut state.phase,
                    state.connection_generation,
                )
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Login(flow)),
                    )
                })
                .map_err(Error::Login),
            Self::OpenTcp(open_tcp) => open_tcp
                .serial(
                    state.timer,
                    &mut state.adapter,
                    state.transfer_length,
                    &mut state.phase,
                    &mut state.sockets,
                    state.connection_generation,
                )
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::OpenTcp(flow)),
                    )
                })
                .map_err(Error::OpenTcp),
            Self::OpenUdp(open_udp) => open_udp
                .serial(
                    state.timer,
                    &mut state.adapter,
                    state.transfer_length,
                    &mut state.phase,
                    &mut state.sockets,
                    state.connection_generation,
                )
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::OpenUdp(flow)),
                    )
                })
                .map_err(Error::OpenUdp),
            Self::WriteConfig(write_config) => write_config
                .serial(state.timer, &mut state.adapter, state.transfer_length)
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::WriteConfig(flow)),
                    )
                })
                .map_err(Error::WriteConfig),
            Self::Status(status) => status
                .serial(state.timer, &mut state.adapter, &mut state.phase)
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Status(flow)),
                    )
                })
                .map_err(Error::Status),
            Self::Idle(idle) => idle
                .serial(state.timer, &mut state.phase)
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Idle(flow)),
                    )
                })
                .map_err(Error::Idle),
        }
    }
}
