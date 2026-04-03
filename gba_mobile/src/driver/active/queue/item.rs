#![allow(private_interfaces)]

use super::super::{Flow, Phase, State};
use crate::{ArrayVec, Digit, Timer, socket};
use core::{
    fmt,
    fmt::{Debug, Formatter},
};

pub(crate) trait SocketSubItem<Socket1, Socket2, const INDEX: usize>: Debug
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
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
    ) -> Option<Flow<Socket1, Socket2>>;
}

pub(crate) trait ConnectionSubItem<Socket1, Socket2>
where
    Socket1: socket::slot::Sealed,
    Socket2: socket::slot::Sealed,
{
    fn connect(
        phone_number: &ArrayVec<Digit, 32>,
        state: &State,
        timer: Timer,
    ) -> Option<Flow<Socket1, Socket2>>;

    fn accept(state: &State, timer: Timer) -> Option<Flow<Socket1, Socket2>>;
}

/// An empty item set.
#[derive(Debug)]
pub(crate) enum Empty {}

impl<Socket1, Socket2, const INDEX: usize> SocketSubItem<Socket1, Socket2, INDEX> for Empty
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
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
    ) -> Option<Flow<Socket1, Socket2>> {
        unreachable!()
    }
}

impl<Socket1, Socket2> ConnectionSubItem<Socket1, Socket2> for Empty
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    fn connect(
        _phone_number: &ArrayVec<Digit, 32>,
        _state: &State,
        _timer: Timer,
    ) -> Option<Flow<Socket1, Socket2>> {
        None
    }

    fn accept(_state: &State, _timer: Timer) -> Option<Flow<Socket1, Socket2>> {
        None
    }
}

#[derive(Debug)]
pub(crate) enum Socket {
    Open,
    Close,
    Transfer,
}

impl<Buffer, Socket2> SocketSubItem<socket::Socket<Buffer>, Socket2, 0> for Socket
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
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
    ) -> Option<Flow<socket::Socket<Buffer>, Socket2>> {
        match self {
            Self::Open => {
                if let Phase::LoggedIn {
                    socket_generations,
                    socket_requests,
                    ..
                } = &mut state.phase
                {
                    socket_requests[0].take().map(|request| match request {
                        (
                            super::super::socket::Request::Dns { domain, port },
                            super::super::socket::Protocol::Tcp,
                        ) => Flow::open_tcp_1_with_dns(
                            state.transfer_length,
                            timer,
                            domain,
                            port,
                            state.connection_generation,
                            socket_generations[0],
                        ),
                        (
                            super::super::socket::Request::Dns { domain, port },
                            super::super::socket::Protocol::Udp,
                        ) => Flow::open_udp_1_with_dns(
                            state.transfer_length,
                            timer,
                            domain,
                            port,
                            state.connection_generation,
                            socket_generations[0],
                        ),
                        (
                            super::super::socket::Request::SocketAddr(addr),
                            super::super::socket::Protocol::Tcp,
                        ) => Flow::open_tcp_1_with_socket_addr(
                            state.transfer_length,
                            timer,
                            addr,
                            state.connection_generation,
                            socket_generations[0],
                        ),
                        (
                            super::super::socket::Request::SocketAddr(addr),
                            super::super::socket::Protocol::Udp,
                        ) => Flow::open_udp_1_with_socket_addr(
                            state.transfer_length,
                            timer,
                            addr,
                            state.connection_generation,
                            socket_generations[0],
                        ),
                    })
                } else {
                    // We are not in the correct phase to open a socket, so we do nothing.
                    None
                }
            }
            Self::Close => todo!(),
            Self::Transfer => Some(Flow::socket_1_transfer_data(
                state.transfer_length,
                timer,
                socket_1,
            )),
        }
    }
}

impl<Buffer, Socket1> SocketSubItem<Socket1, socket::Socket<Buffer>, 1> for Socket
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
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
    ) -> Option<Flow<Socket1, socket::Socket<Buffer>>> {
        match self {
            Self::Open => {
                if let Phase::LoggedIn {
                    socket_generations,
                    socket_requests,
                    ..
                } = &mut state.phase
                {
                    socket_requests[1].take().map(|request| match request {
                        (
                            super::super::socket::Request::Dns { domain, port },
                            super::super::socket::Protocol::Tcp,
                        ) => Flow::open_tcp_2_with_dns(
                            state.transfer_length,
                            timer,
                            domain,
                            port,
                            state.connection_generation,
                            socket_generations[1],
                        ),
                        (
                            super::super::socket::Request::Dns { domain, port },
                            super::super::socket::Protocol::Udp,
                        ) => Flow::open_udp_2_with_dns(
                            state.transfer_length,
                            timer,
                            domain,
                            port,
                            state.connection_generation,
                            socket_generations[1],
                        ),
                        (
                            super::super::socket::Request::SocketAddr(addr),
                            super::super::socket::Protocol::Tcp,
                        ) => Flow::open_tcp_2_with_socket_addr(
                            state.transfer_length,
                            timer,
                            addr,
                            state.connection_generation,
                            socket_generations[1],
                        ),
                        (
                            super::super::socket::Request::SocketAddr(addr),
                            super::super::socket::Protocol::Udp,
                        ) => Flow::open_udp_2_with_socket_addr(
                            state.transfer_length,
                            timer,
                            addr,
                            state.connection_generation,
                            socket_generations[1],
                        ),
                    })
                } else {
                    // We are not in the correct phase to open a socket, so we do nothing.
                    None
                }
            }
            Self::Close => todo!(),
            Self::Transfer => Some(Flow::socket_2_transfer_data(
                state.transfer_length,
                timer,
                socket_2,
            )),
        }
    }
}

impl<Buffer, Socket2> ConnectionSubItem<socket::Socket<Buffer>, Socket2> for Socket
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
{
    fn connect(
        phone_number: &ArrayVec<Digit, 32>,
        state: &State,
        timer: Timer,
    ) -> Option<Flow<socket::Socket<Buffer>, Socket2>> {
        Some(Flow::connect(
            state.transfer_length,
            timer,
            state.adapter,
            phone_number.clone(),
            state.connection_generation,
        ))
    }

    fn accept(state: &State, timer: Timer) -> Option<Flow<socket::Socket<Buffer>, Socket2>> {
        Some(Flow::accept(state.transfer_length, timer))
    }
}

pub(in super::super) enum Item<Socket1, Socket2>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    Start,
    End,
    Reset,

    Connect,
    Disconnect,

    Socket1(Socket1::Socket1Item<Socket2>),
    Socket2(Socket2::Socket2Item<Socket1>),

    WriteConfig,

    Status,
    Idle,
}

impl<Socket1, Socket2> Debug for Item<Socket1, Socket2>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
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

            Self::WriteConfig => formatter.write_str("WriteConfig"),

            Self::Status => formatter.write_str("Status"),
            Self::Idle => formatter.write_str("Idle"),
        }
    }
}
