#![allow(private_interfaces)]

use super::super::{Flow, Phase, State};
use crate::{
    ArrayVec, Config, Digit, Timer,
    config::{self, NoConfig},
    dns::{self, NoDns},
    socket,
};
use core::{
    fmt,
    fmt::{Debug, Formatter},
};

pub(crate) trait SocketSubItem<Socket1, Socket2, Dns, Config, const INDEX: usize>:
    Debug
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn open() -> Self;
    fn close() -> Self;
    fn transfer() -> Self;

    fn next_flow(
        self,
        state: &mut State,
        timer: Timer,
        socket_1: &mut Socket1,
        socket_2: &mut Socket2,
    ) -> Option<Flow<Socket1, Socket2, Dns, Config>>;
}

pub(crate) trait ConnectionSubItem<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn connect(
        phone_number: &ArrayVec<Digit, 32>,
        state: &State,
        timer: Timer,
    ) -> Option<Flow<Socket1, Socket2, Dns, Config>>;

    fn accept(state: &State, timer: Timer) -> Option<Flow<Socket1, Socket2, Dns, Config>>;
}

pub(crate) trait DnsSubItem<Socket1, Socket2, Dns, Config>: Debug
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn dns() -> Self;

    fn flow(
        self,
        dns: &Dns,
        state: &State,
        timer: Timer,
    ) -> Option<Flow<Socket1, Socket2, Dns, Config>>;
}

pub(crate) trait ConfigSubItem<Socket1, Socket2, Dns, Config>: Debug
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn write_config() -> Self;

    fn flow(
        self,
        config: &Config,
        state: &State,
        timer: Timer,
    ) -> Option<Flow<Socket1, Socket2, Dns, Config>>;
}

/// An empty item set.
#[derive(Debug)]
pub(crate) enum Empty {}

impl<Socket1, Socket2, Dns, Config, const INDEX: usize>
    SocketSubItem<Socket1, Socket2, Dns, Config, INDEX> for Empty
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn open() -> Self {
        unreachable!()
    }

    fn close() -> Self {
        unreachable!()
    }

    fn transfer() -> Self {
        unreachable!()
    }

    fn next_flow(
        self,
        _state: &mut State,
        _timer: Timer,
        _socket_1: &mut Socket1,
        _socket_2: &mut Socket2,
    ) -> Option<Flow<Socket1, Socket2, Dns, Config>> {
        unreachable!()
    }
}

impl<Socket1, Socket2, Dns, Config> ConnectionSubItem<Socket1, Socket2, Dns, Config> for Empty
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn connect(
        _phone_number: &ArrayVec<Digit, 32>,
        _state: &State,
        _timer: Timer,
    ) -> Option<Flow<Socket1, Socket2, Dns, Config>> {
        None
    }

    fn accept(_state: &State, _timer: Timer) -> Option<Flow<Socket1, Socket2, Dns, Config>> {
        None
    }
}

impl<Socket1, Socket2, Config> DnsSubItem<Socket1, Socket2, NoDns, Config> for Empty
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
    Config: config::Sealed,
{
    fn dns() -> Self {
        unreachable!()
    }

    fn flow(
        self,
        _dns: &NoDns,
        _state: &State,
        _timer: Timer,
    ) -> Option<Flow<Socket1, Socket2, NoDns, Config>> {
        None
    }
}

impl<Socket1, Socket2, Dns> ConfigSubItem<Socket1, Socket2, Dns, NoConfig> for Empty
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Sealed,
{
    fn write_config() -> Self {
        unreachable!()
    }

    fn flow(
        self,
        _config: &NoConfig,
        _state: &State,
        _timer: Timer,
    ) -> Option<Flow<Socket1, Socket2, Dns, NoConfig>> {
        None
    }
}

#[derive(Debug)]
pub(crate) enum Socket {
    Open,
    Close,
    Transfer,
}

impl<Buffer, Socket2, Dns, Config> SocketSubItem<socket::Socket<Buffer>, Socket2, Dns, Config, 0>
    for Socket
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn open() -> Self {
        Self::Open
    }

    fn close() -> Self {
        Self::Close
    }

    fn transfer() -> Self {
        Self::Transfer
    }

    fn next_flow(
        self,
        state: &mut State,
        timer: Timer,
        socket_1: &mut socket::Socket<Buffer>,
        _socket_2: &mut Socket2,
    ) -> Option<Flow<socket::Socket<Buffer>, Socket2, Dns, Config>> {
        match self {
            Self::Open => {
                if let Phase::LoggedIn {
                    socket_generations,
                    socket_requests,
                    ..
                } = &mut state.phase
                {
                    socket_requests[0]
                        .take()
                        .map(|(socket_addr, protocol)| match protocol {
                            socket::Protocol::Tcp => Flow::open_tcp_1(
                                state.transfer_length,
                                timer,
                                socket_addr,
                                state.connection_generation,
                                socket_generations[0],
                            ),
                            socket::Protocol::Udp => Flow::open_udp_1(
                                state.transfer_length,
                                timer,
                                socket_addr,
                                state.connection_generation,
                                socket_generations[0],
                            ),
                        })
                } else {
                    // We are not in the correct phase to open a socket, so we do nothing.
                    None
                }
            }
            Self::Close => {
                if let Phase::LoggedIn {
                    socket_protocols, ..
                } = &mut state.phase
                {
                    Some(match socket_protocols[0] {
                        socket::Protocol::Tcp => {
                            Flow::close_tcp_1(state.transfer_length, timer, socket_1.id)
                        }
                        socket::Protocol::Udp => {
                            Flow::close_udp_1(state.transfer_length, timer, socket_1.id)
                        }
                    })
                } else {
                    // We are not in the correct phase to close a socket, so we do nothing.
                    None
                }
            }
            Self::Transfer => Some(Flow::socket_1_transfer_data(
                state.transfer_length,
                timer,
                socket_1,
            )),
        }
    }
}

impl<Buffer, Socket1, Dns, Config> SocketSubItem<Socket1, socket::Socket<Buffer>, Dns, Config, 1>
    for Socket
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn open() -> Self {
        Self::Open
    }

    fn close() -> Self {
        Self::Close
    }

    fn transfer() -> Self {
        Self::Transfer
    }

    fn next_flow(
        self,
        state: &mut State,
        timer: Timer,
        _socket_1: &mut Socket1,
        socket_2: &mut socket::Socket<Buffer>,
    ) -> Option<Flow<Socket1, socket::Socket<Buffer>, Dns, Config>> {
        match self {
            Self::Open => {
                if let Phase::LoggedIn {
                    socket_generations,
                    socket_requests,
                    ..
                } = &mut state.phase
                {
                    socket_requests[1]
                        .take()
                        .map(|(socket_addr, protocol)| match protocol {
                            socket::Protocol::Tcp => Flow::open_tcp_2(
                                state.transfer_length,
                                timer,
                                socket_addr,
                                state.connection_generation,
                                socket_generations[1],
                            ),
                            socket::Protocol::Udp => Flow::open_udp_2(
                                state.transfer_length,
                                timer,
                                socket_addr,
                                state.connection_generation,
                                socket_generations[1],
                            ),
                        })
                } else {
                    // We are not in the correct phase to open a socket, so we do nothing.
                    None
                }
            }
            Self::Close => {
                if let Phase::LoggedIn {
                    socket_protocols, ..
                } = &mut state.phase
                {
                    Some(match socket_protocols[1] {
                        socket::Protocol::Tcp => {
                            Flow::close_tcp_2(state.transfer_length, timer, socket_2.id)
                        }
                        socket::Protocol::Udp => {
                            Flow::close_udp_2(state.transfer_length, timer, socket_2.id)
                        }
                    })
                } else {
                    // We are not in the correct phase to close a socket, so we do nothing.
                    None
                }
            }
            Self::Transfer => Some(Flow::socket_2_transfer_data(
                state.transfer_length,
                timer,
                socket_2,
            )),
        }
    }
}

impl<Buffer, Socket2, Dns, Config> ConnectionSubItem<socket::Socket<Buffer>, Socket2, Dns, Config>
    for Socket
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Sealed,
    Config: config::Sealed,
{
    fn connect(
        phone_number: &ArrayVec<Digit, 32>,
        state: &State,
        timer: Timer,
    ) -> Option<Flow<socket::Socket<Buffer>, Socket2, Dns, Config>> {
        Some(Flow::connect(
            state.transfer_length,
            timer,
            state.adapter,
            phone_number.clone(),
            state.connection_generation,
        ))
    }

    fn accept(
        state: &State,
        timer: Timer,
    ) -> Option<Flow<socket::Socket<Buffer>, Socket2, Dns, Config>> {
        Some(Flow::accept(state.transfer_length, timer))
    }
}

#[derive(Debug)]
pub(crate) struct Dns;

impl<Socket1, Socket2, Config, const MAX_LEN: usize>
    DnsSubItem<Socket1, Socket2, dns::Dns<MAX_LEN>, Config> for Dns
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
    Config: config::Sealed,
{
    fn dns() -> Self {
        Self
    }

    fn flow(
        self,
        dns: &dns::Dns<MAX_LEN>,
        state: &State,
        timer: Timer,
    ) -> Option<Flow<Socket1, Socket2, dns::Dns<MAX_LEN>, Config>> {
        if let dns::State::Request(name) = &dns.state {
            Some(Flow::dns(
                state.transfer_length,
                timer,
                name.clone(),
                dns.generation,
            ))
        } else {
            // There is no request in the actual DNS object, so there's nothing to send.
            None
        }
    }
}

#[derive(Debug)]
pub(crate) struct WriteConfig;

impl<Socket1, Socket2, Dns, Format> ConfigSubItem<Socket1, Socket2, Dns, Config<Format>>
    for WriteConfig
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Sealed,
    Format: config::Format,
{
    fn write_config() -> Self {
        Self
    }

    fn flow(
        self,
        config: &Config<Format>,
        state: &State,
        timer: Timer,
    ) -> Option<Flow<Socket1, Socket2, Dns, Config<Format>>> {
        Flow::write_config(state.transfer_length, timer, config)
    }
}

pub(in super::super) enum Item<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    Start,
    End,
    Reset,

    Connect,
    Disconnect,

    Socket1(Socket1::Socket1Item<Socket2, Dns, Config>),
    Socket2(Socket2::Socket2Item<Socket1, Dns, Config>),
    Dns(Dns::Item<Socket1, Socket2, Config>),
    Config(Config::Item<Socket1, Socket2, Dns>),

    Status,
    Idle,
}

impl<Socket1, Socket2, Dns, Config> Debug for Item<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start => formatter.write_str("Start"),
            Self::End => formatter.write_str("End"),
            Self::Reset => formatter.write_str("Reset"),

            Self::Connect => formatter.write_str("Connect"),
            Self::Disconnect => formatter.write_str("Disconnect"),

            Self::Socket1(item) => formatter.debug_tuple("Socket1").field(item).finish(),
            Self::Socket2(item) => formatter.debug_tuple("Socket2").field(item).finish(),
            Self::Dns(item) => formatter.debug_tuple("Dns").field(item).finish(),
            Self::Config(item) => formatter.debug_tuple("Config").field(item).finish(),

            Self::Status => formatter.write_str("Status"),
            Self::Idle => formatter.write_str("Idle"),
        }
    }
}
