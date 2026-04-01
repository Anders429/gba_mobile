#![allow(private_interfaces)]

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
mod transfer_data;
mod write_config;

use core::{
    fmt::Debug,
    net::{Ipv4Addr, SocketAddrV4},
};

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{Phase, Queue, State, StateChange};
use crate::{
    ArrayVec, Digit, Generation, Socket, Timer, driver::Adapter, mmio::serial::TransferLength,
    socket, socket::NoSocket,
};
use accept::Accept;
use connect::Connect;
use either::Either;
use embedded_io::{Read, Write};
use end::End;
use idle::Idle;
use login::Login;
use open_tcp::OpenTcp;
use open_udp::OpenUdp;
use reset::Reset;
use start::Start;
use status::Status;
use transfer_data::TransferData;
use write_config::WriteConfig;

pub(crate) trait SubFlowWithSocket<Socket>: Sized + Debug {
    fn vblank(self) -> Result<Self, Timeout>;
    fn timer(&mut self);
    fn serial(
        self,
        state: &mut State,
        timer: Timer,
        socket: &mut Socket,
    ) -> Result<Option<Self>, Error>;
}

/// An empty flow.
///
/// Used when certain flows are not available due to the configuration of the driver.
#[derive(Debug)]
pub(crate) enum Empty {}

impl SubFlowWithSocket<NoSocket> for Empty {
    fn vblank(self) -> Result<Self, Timeout> {
        unreachable!()
    }

    fn timer(&mut self) {
        unreachable!()
    }

    fn serial(
        self,
        _state: &mut State,
        _timer: Timer,
        _socket: &mut NoSocket,
    ) -> Result<Option<Self>, Error> {
        unreachable!()
    }
}

#[derive(Debug)]
pub(crate) enum ConnectionFlow {
    Accept(Accept),
    Connect(Connect),
}

impl<Buffer> SubFlowWithSocket<Socket<Buffer>> for ConnectionFlow {
    fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::Accept(accept) => accept.vblank().map(Self::Accept).map_err(Timeout::Accept),
            Self::Connect(connect) => connect
                .vblank()
                .map(Self::Connect)
                .map_err(Timeout::Connect),
        }
    }

    fn timer(&mut self) {
        match self {
            Self::Accept(accept) => accept.timer(),
            Self::Connect(connect) => connect.timer(),
        }
    }

    fn serial(
        self,
        state: &mut State,
        timer: Timer,
        socket: &mut Socket<Buffer>,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::Accept(accept) => accept
                .serial(timer, &mut state.adapter, &mut state.phase, socket)
                .map(|flow| flow.map(Self::Accept))
                .map_err(Error::Accept),
            Self::Connect(connect) => connect
                .serial(
                    timer,
                    &mut state.adapter,
                    &mut state.phase,
                    socket,
                    state.connection_generation,
                )
                .map(|flow| flow.map(Self::Connect))
                .map_err(Error::Connect),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SocketFlow<const INDEX: usize> {
    OpenTcp(OpenTcp<INDEX>),
    OpenUdp(OpenUdp<INDEX>),
    TransferData(TransferData),
}

impl<Buffer, const INDEX: usize> SubFlowWithSocket<Socket<Buffer>> for SocketFlow<INDEX>
where
    Buffer: Write,
{
    fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::OpenTcp(open_tcp) => open_tcp
                .vblank()
                .map(Self::OpenTcp)
                .map_err(Timeout::OpenTcp),
            Self::OpenUdp(open_udp) => open_udp
                .vblank()
                .map(Self::OpenUdp)
                .map_err(Timeout::OpenUdp),
            Self::TransferData(transfer_data) => transfer_data
                .vblank()
                .map(Self::TransferData)
                .map_err(Timeout::TransferData),
        }
    }

    fn timer(&mut self) {
        match self {
            Self::OpenTcp(open_tcp) => open_tcp.timer(),
            Self::OpenUdp(open_udp) => open_udp.timer(),
            Self::TransferData(transfer_data) => transfer_data.timer(),
        }
    }

    fn serial(
        self,
        state: &mut State,
        timer: Timer,
        socket: &mut Socket<Buffer>,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::OpenTcp(open_tcp) => open_tcp
                .serial(
                    timer,
                    &mut state.adapter,
                    state.transfer_length,
                    &mut state.phase,
                    socket,
                    state.connection_generation,
                )
                .map(|flow| flow.map(Self::OpenTcp))
                .map_err(Error::OpenTcp),
            Self::OpenUdp(open_udp) => open_udp
                .serial(
                    timer,
                    &mut state.adapter,
                    state.transfer_length,
                    &mut state.phase,
                    socket,
                    state.connection_generation,
                )
                .map(|flow| flow.map(Self::OpenUdp))
                .map_err(Error::OpenUdp),
            Self::TransferData(transfer_data) => transfer_data
                .serial(timer, &mut state.adapter, socket)
                .map(|flow| flow.map(Self::TransferData))
                .map_err(Error::TransferData),
        }
    }
}

#[derive(Debug)]
pub(super) enum Flow<Socket1, Socket2>
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
{
    Start(Start),
    End(End),
    Reset(Reset),

    Login(Login),

    Connection(Socket1::ConnectionFlow),

    Socket1(Socket1::SocketFlow<0>),
    Socket2(Socket2::SocketFlow<1>),

    WriteConfig(WriteConfig),

    Status(Status),
    Idle(Idle),
}

impl<Socket1, Socket2> Flow<Socket1, Socket2>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub(super) fn start(transfer_length: TransferLength) -> Self {
        Self::Start(Start::new(transfer_length))
    }

    pub(super) fn end(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::End(End::new(transfer_length, timer))
    }

    pub(super) fn reset(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::Reset(Reset::new(transfer_length, timer))
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
            Self::Login(login) => login.vblank().map(Self::Login).map_err(Timeout::Login),
            Self::Connection(connection) => connection.vblank().map(Self::Connection),
            Self::Socket1(socket_1) => socket_1.vblank().map(Self::Socket1),
            Self::Socket2(socket_2) => socket_2.vblank().map(Self::Socket2),
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
            Self::Login(login) => login.timer(),
            Self::Connection(connection) => connection.timer(),
            Self::Socket1(socket_1) => socket_1.timer(),
            Self::Socket2(socket_2) => socket_2.timer(),
            Self::WriteConfig(write_config) => write_config.timer(),
            Self::Status(status) => status.timer(),
            Self::Idle(idle) => idle.timer(),
        }
    }

    pub(super) fn serial(
        self,
        state: &mut State,
        queue: &mut Queue<Socket1, Socket2>,
        timer: Timer,
        socket_1: &mut Socket1,
        socket_2: &mut Socket2,
    ) -> Result<Either<Self, StateChange>, Error> {
        match self {
            Self::Start(start) => start
                .serial(
                    timer,
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
                .serial(timer, &mut state.adapter, &mut state.transfer_length)
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
                    timer,
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
            Self::Login(login) => login
                .serial(
                    timer,
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
            Self::Connection(connection) => connection.serial(state, timer, socket_1).map(|flow| {
                if let Some(flow) = flow {
                    Either::Left(Self::Connection(flow))
                } else {
                    Either::Right(StateChange::StillActive)
                }
            }),
            Self::Socket1(socket) => socket.serial(state, timer, socket_1).map(|flow| {
                if let Some(flow) = flow {
                    Either::Left(Self::Socket1(flow))
                } else {
                    Either::Right(StateChange::StillActive)
                }
            }),
            Self::Socket2(socket) => socket.serial(state, timer, socket_2).map(|flow| {
                if let Some(flow) = flow {
                    Either::Left(Self::Socket2(flow))
                } else {
                    Either::Right(StateChange::StillActive)
                }
            }),
            Self::WriteConfig(write_config) => write_config
                .serial(timer, &mut state.adapter, state.transfer_length)
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::WriteConfig(flow)),
                    )
                })
                .map_err(Error::WriteConfig),
            Self::Status(status) => status
                .serial(timer, &mut state.adapter, &mut state.phase)
                .map(|flow| {
                    flow.map_or_else(
                        || Either::Right(StateChange::StillActive),
                        |flow| Either::Left(Self::Status(flow)),
                    )
                })
                .map_err(Error::Status),
            Self::Idle(idle) => idle
                .serial(timer, &mut state.phase)
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

impl<Buffer, Socket2> Flow<Socket<Buffer>, Socket2>
where
    Buffer: Read + Write,
    Socket2: socket::Slot,
{
    pub(super) fn accept(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::Connection(ConnectionFlow::Accept(Accept::new(transfer_length, timer)))
    }

    pub(super) fn connect(
        transfer_length: TransferLength,
        timer: Timer,
        adapter: Adapter,
        phone_number: ArrayVec<Digit, 32>,
        connection_generation: Generation,
    ) -> Self {
        Self::Connection(ConnectionFlow::Connect(Connect::new(
            transfer_length,
            timer,
            adapter,
            phone_number,
            connection_generation,
        )))
    }

    pub(super) fn open_tcp_1_with_dns(
        transfer_length: TransferLength,
        timer: Timer,
        domain: ArrayVec<u8, 255>,
        port: u16,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket1(SocketFlow::OpenTcp(OpenTcp::with_dns(
            transfer_length,
            timer,
            domain,
            port,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn open_tcp_1_with_socket_addr(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket1(SocketFlow::OpenTcp(OpenTcp::with_socket_addr(
            transfer_length,
            timer,
            addr,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn open_udp_1_with_dns(
        transfer_length: TransferLength,
        timer: Timer,
        domain: ArrayVec<u8, 255>,
        port: u16,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket1(SocketFlow::OpenUdp(OpenUdp::with_dns(
            transfer_length,
            timer,
            domain,
            port,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn open_udp_1_with_socket_addr(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket1(SocketFlow::OpenUdp(OpenUdp::with_socket_addr(
            transfer_length,
            timer,
            addr,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn socket_1_transfer_data(
        transfer_length: TransferLength,
        timer: Timer,
        socket: &mut Socket<Buffer>,
    ) -> Self {
        Self::Socket1(SocketFlow::TransferData(TransferData::new(
            transfer_length,
            timer,
            socket,
        )))
    }
}

impl<Buffer, Socket1> Flow<Socket1, Socket<Buffer>>
where
    Buffer: Read + Write,
    Socket1: socket::Slot,
{
    pub(super) fn open_tcp_2_with_dns(
        transfer_length: TransferLength,
        timer: Timer,
        domain: ArrayVec<u8, 255>,
        port: u16,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket2(SocketFlow::OpenTcp(OpenTcp::with_dns(
            transfer_length,
            timer,
            domain,
            port,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn open_tcp_2_with_socket_addr(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket2(SocketFlow::OpenTcp(OpenTcp::with_socket_addr(
            transfer_length,
            timer,
            addr,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn open_udp_2_with_dns(
        transfer_length: TransferLength,
        timer: Timer,
        domain: ArrayVec<u8, 255>,
        port: u16,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket2(SocketFlow::OpenUdp(OpenUdp::with_dns(
            transfer_length,
            timer,
            domain,
            port,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn open_udp_2_with_socket_addr(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket2(SocketFlow::OpenUdp(OpenUdp::with_socket_addr(
            transfer_length,
            timer,
            addr,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn socket_2_transfer_data(
        transfer_length: TransferLength,
        timer: Timer,
        socket: &mut Socket<Buffer>,
    ) -> Self {
        Self::Socket2(SocketFlow::TransferData(TransferData::new(
            transfer_length,
            timer,
            socket,
        )))
    }
}
