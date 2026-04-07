#![allow(private_interfaces)]

mod accept;
mod connect;
mod disconnect;
mod dns;
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
    convert::Infallible,
    fmt::Debug,
    net::{Ipv4Addr, SocketAddrV4},
};

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{Queue, State};
use crate::{
    ArrayVec, Digit, Generation, Socket, Timer,
    dns::NoDns,
    driver::Adapter,
    mmio::serial::TransferLength,
    socket::{self, NoSocket},
};
use accept::Accept;
use connect::Connect;
use disconnect::Disconnect;
use dns::Dns;
use either::Either;
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

pub(crate) trait SocketSubFlow<Socket>: Sized + Debug {
    type Error: Clone + core::error::Error + 'static;

    fn vblank(self) -> Result<Self, Timeout>;
    fn timer(&mut self);
    fn serial(
        self,
        state: &mut State,
        timer: Timer,
        socket: &mut Socket,
    ) -> Result<Option<Self>, Self::Error>;
}

pub(crate) trait DnsSubFlow<Dns>: Sized + Debug {
    type Error: Clone + core::error::Error + 'static;

    fn vblank(self) -> Result<Self, Timeout>;
    fn timer(&mut self);
    fn serial(
        self,
        state: &mut State,
        timer: Timer,
        dns: &mut Dns,
    ) -> Result<Option<Self>, Self::Error>;
}

/// An empty flow.
///
/// Used when certain flows are not available due to the configuration of the driver.
#[derive(Debug)]
pub(crate) enum Empty {}

impl SocketSubFlow<NoSocket> for Empty {
    type Error = Infallible;

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
    ) -> Result<Option<Self>, Self::Error> {
        unreachable!()
    }
}

impl DnsSubFlow<NoDns> for Empty {
    type Error = Infallible;

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
        _dns: &mut NoDns,
    ) -> Result<Option<Self>, Self::Error> {
        unreachable!()
    }
}

#[derive(Debug)]
pub(crate) enum ConnectionFlow {
    Accept(Accept),
    Connect(Connect),
}

impl<Buffer> SocketSubFlow<Socket<Buffer>> for ConnectionFlow {
    type Error = error::Connection;

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
    ) -> Result<Option<Self>, Self::Error> {
        match self {
            Self::Accept(accept) => accept
                .serial(timer, &mut state.adapter, &mut state.phase, socket)
                .map(|flow| flow.map(Self::Accept))
                .map_err(error::Connection::Accept),
            Self::Connect(connect) => connect
                .serial(
                    timer,
                    &mut state.adapter,
                    &mut state.phase,
                    socket,
                    state.connection_generation,
                )
                .map(|flow| flow.map(Self::Connect))
                .map_err(error::Connection::Connect),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SocketFlow<const INDEX: usize> {
    OpenTcp(OpenTcp<INDEX>),
    OpenUdp(OpenUdp<INDEX>),
    TransferData(TransferData),
}

impl<Buffer, const INDEX: usize> SocketSubFlow<Socket<Buffer>> for SocketFlow<INDEX>
where
    Buffer: socket::Buffer,
{
    type Error = error::Socket<Buffer::WriteError>;

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
    ) -> Result<Option<Self>, Self::Error> {
        match self {
            Self::OpenTcp(open_tcp) => open_tcp
                .serial(
                    timer,
                    &mut state.adapter,
                    &mut state.phase,
                    socket,
                    state.connection_generation,
                )
                .map(|flow| flow.map(Self::OpenTcp))
                .map_err(error::Socket::OpenTcp),
            Self::OpenUdp(open_udp) => open_udp
                .serial(
                    timer,
                    &mut state.adapter,
                    &mut state.phase,
                    socket,
                    state.connection_generation,
                )
                .map(|flow| flow.map(Self::OpenUdp))
                .map_err(error::Socket::OpenUdp),
            Self::TransferData(transfer_data) => transfer_data
                .serial(timer, &mut state.adapter, state.transfer_length, socket)
                .map(|flow| flow.map(Self::TransferData))
                .map_err(error::Socket::TransferData),
        }
    }
}

#[derive(Debug)]
pub(crate) struct DnsFlow<const MAX_LEN: usize>(Dns<MAX_LEN>);

impl<const MAX_LEN: usize> DnsSubFlow<crate::Dns<MAX_LEN>> for DnsFlow<MAX_LEN> {
    type Error = error::Dns<MAX_LEN>;

    fn vblank(self) -> Result<Self, Timeout> {
        self.0.vblank().map(DnsFlow).map_err(Timeout::Dns)
    }

    fn timer(&mut self) {
        self.0.timer();
    }

    fn serial(
        self,
        state: &mut State,
        timer: Timer,
        dns: &mut crate::Dns<MAX_LEN>,
    ) -> Result<Option<Self>, Self::Error> {
        self.0
            .serial(timer, &mut state.adapter, dns)
            .map(|flow| flow.map(Self))
            .map_err(error::Dns::Dns)
    }
}

#[derive(Debug)]
pub(super) enum Flow<Socket1, Socket2, Dns>
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
    Dns: crate::dns::Sealed,
{
    Start(Start),
    End(End),
    Reset(Reset),

    Login(Login),

    Connection(Socket1::ConnectionFlow),
    Disconnect(Disconnect),

    Socket1(Socket1::SocketFlow<0>),
    Socket2(Socket2::SocketFlow<1>),
    Dns(Dns::Flow),

    WriteConfig(WriteConfig),

    Status(Status),
    Idle(Idle),
}

impl<Socket1, Socket2, Dns> Flow<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
{
    pub(super) fn start(transfer_length: TransferLength, link_generation: Generation) -> Self {
        Self::Start(Start::new(transfer_length, link_generation))
    }

    pub(super) fn end(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::End(End::new(transfer_length, timer))
    }

    pub(super) fn reset(
        transfer_length: TransferLength,
        timer: Timer,
        link_generation: Generation,
    ) -> Self {
        Self::Reset(Reset::new(transfer_length, timer, link_generation))
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

    pub(super) fn disconnect(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::Disconnect(Disconnect::new(transfer_length, timer))
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

    /// Only returns `None` if the active session is being ended.
    pub(super) fn vblank(self) -> Result<Option<Self>, Timeout> {
        match self {
            Self::Start(start) => start
                .vblank()
                .map(|flow| Some(Self::Start(flow)))
                .map_err(Timeout::Start),
            Self::End(end) => end
                .vblank()
                .map(|flow| flow.map(Self::End))
                .map_err(Timeout::End),
            Self::Reset(reset) => reset
                .vblank()
                .map(|flow| Some(Self::Reset(flow)))
                .map_err(Timeout::Reset),
            Self::Login(login) => login
                .vblank()
                .map(|flow| Some(Self::Login(flow)))
                .map_err(Timeout::Login),
            Self::Connection(connection) => {
                connection.vblank().map(|flow| Some(Self::Connection(flow)))
            }
            Self::Disconnect(disconnect) => disconnect
                .vblank()
                .map(|flow| Some(Self::Disconnect(flow)))
                .map_err(Timeout::Disconnect),
            Self::Socket1(socket_1) => socket_1.vblank().map(|flow| Some(Self::Socket1(flow))),
            Self::Socket2(socket_2) => socket_2.vblank().map(|flow| Some(Self::Socket2(flow))),
            Self::Dns(dns) => dns.vblank().map(|flow| Some(Self::Dns(flow))),
            Self::WriteConfig(write_config) => write_config
                .vblank()
                .map(|flow| Some(Self::WriteConfig(flow)))
                .map_err(Timeout::WriteConfig),
            Self::Status(status) => status
                .vblank()
                .map(|flow| Some(Self::Status(flow)))
                .map_err(Timeout::Status),
            Self::Idle(idle) => idle
                .vblank()
                .map(|flow| Some(Self::Idle(flow)))
                .map_err(Timeout::Idle),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::Start(start) => start.timer(),
            Self::End(end) => end.timer(),
            Self::Reset(reset) => reset.timer(),
            Self::Login(login) => login.timer(),
            Self::Connection(connection) => connection.timer(),
            Self::Disconnect(disconnect) => disconnect.timer(),
            Self::Socket1(socket_1) => socket_1.timer(),
            Self::Socket2(socket_2) => socket_2.timer(),
            Self::Dns(dns) => dns.timer(),
            Self::WriteConfig(write_config) => write_config.timer(),
            Self::Status(status) => status.timer(),
            Self::Idle(idle) => idle.timer(),
        }
    }

    pub(super) fn serial(
        self,
        state: &mut State,
        queue: &mut Queue<Socket1, Socket2, Dns>,
        timer: Timer,
        link_generation: Generation,
        socket_1: &mut Socket1,
        socket_2: &mut Socket2,
        dns: &mut Dns,
    ) -> Result<Option<Self>, Error<Socket1, Socket2, Dns>> {
        match self {
            Self::Start(start) => start
                .serial(
                    timer,
                    &mut state.adapter,
                    &mut state.transfer_length,
                    &mut state.phase,
                    &mut state.config,
                    link_generation,
                )
                .map(|response| match response {
                    Either::Left(start) => Some(Self::Start(start)),
                    Either::Right(response) => {
                        match response {
                            start::Response::Success => {}
                            start::Response::AlreadyActive => {
                                queue.set_end();
                                queue.set_start();
                            }
                        }
                        None
                    }
                })
                .map_err(Error::Start),
            Self::End(end) => end
                .serial(timer, &mut state.adapter, &mut state.transfer_length)
                .map(|flow| Some(Self::End(flow)))
                .map_err(Error::End),
            Self::Reset(reset) => reset
                .serial(
                    timer,
                    &mut state.adapter,
                    &mut state.transfer_length,
                    &mut state.phase,
                    &mut state.config,
                    link_generation,
                )
                .map(|flow| flow.map(Self::Reset))
                .map_err(Error::Reset),
            Self::Login(login) => login
                .serial(
                    timer,
                    &mut state.adapter,
                    state.transfer_length,
                    &mut state.phase,
                    state.connection_generation,
                )
                .map(|flow| flow.map(Self::Login))
                .map_err(Error::Login),
            Self::Connection(connection) => connection
                .serial(state, timer, socket_1)
                .map(|flow| flow.map(Self::Connection))
                .map_err(Error::Connection),
            Self::Disconnect(disconnect) => disconnect
                .serial(timer, &mut state.adapter)
                .map(|flow| flow.map(Self::Disconnect))
                .map_err(Error::Disconnect),
            Self::Socket1(socket) => socket
                .serial(state, timer, socket_1)
                .map(|flow| flow.map(Self::Socket1))
                .map_err(Error::Socket1),
            Self::Socket2(socket) => socket
                .serial(state, timer, socket_2)
                .map(|flow| flow.map(Self::Socket2))
                .map_err(Error::Socket2),
            Self::Dns(flow) => flow
                .serial(state, timer, dns)
                .map(|flow| flow.map(Self::Dns))
                .map_err(Error::Dns),
            Self::WriteConfig(write_config) => write_config
                .serial(timer, &mut state.adapter, state.transfer_length)
                .map(|flow| flow.map(Self::WriteConfig))
                .map_err(Error::WriteConfig),
            Self::Status(status) => status
                .serial(timer, &mut state.adapter, &mut state.phase)
                .map(|flow| flow.map(Self::Status))
                .map_err(Error::Status),
            Self::Idle(idle) => idle
                .serial(timer, &mut state.phase)
                .map(|flow| flow.map(Self::Idle))
                .map_err(Error::Idle),
        }
    }
}

impl<Buffer, Socket2, Dns> Flow<Socket<Buffer>, Socket2, Dns>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: crate::dns::Sealed,
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

    pub(super) fn open_tcp_1(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket1(SocketFlow::OpenTcp(OpenTcp::new(
            transfer_length,
            timer,
            addr,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn open_udp_1(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket1(SocketFlow::OpenUdp(OpenUdp::new(
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

impl<Buffer, Socket1, Dns> Flow<Socket1, Socket<Buffer>, Dns>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: crate::dns::Sealed,
{
    pub(super) fn open_tcp_2(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket2(SocketFlow::OpenTcp(OpenTcp::new(
            transfer_length,
            timer,
            addr,
            connection_generation,
            socket_generation,
        )))
    }

    pub(super) fn open_udp_2(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self::Socket2(SocketFlow::OpenUdp(OpenUdp::new(
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

impl<Socket1, Socket2, const MAX_LEN: usize> Flow<Socket1, Socket2, crate::Dns<MAX_LEN>>
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
{
    pub(super) fn dns(
        transfer_length: TransferLength,
        timer: Timer,
        name: ArrayVec<u8, MAX_LEN>,
        dns_generation: Generation,
    ) -> Self {
        Self::Dns(DnsFlow(Dns::new(
            transfer_length,
            timer,
            name,
            dns_generation,
        )))
    }
}
