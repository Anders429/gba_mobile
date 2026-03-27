use crate::ArrayVec;
use core::{
    fmt::{self, Display, Formatter},
    net::SocketAddrV4,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub(in crate::driver) struct Id(pub(super) u8);

impl Id {
    pub(super) const P2P: Id = Id(0xff);
}

impl From<u8> for Id {
    fn from(byte: u8) -> Self {
        Id(byte)
    }
}

impl Display for Id {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{:#04x}", self.0)
    }
}

#[derive(Debug)]
pub(in crate::driver) enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug)]
pub(super) enum Request {
    Dns {
        domain: ArrayVec<u8, 255>,
        port: u16,
    },
    SocketAddr(SocketAddrV4),
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum Failure {
    Dns,
    Connect,
    ConnectionFailed,
}

impl Display for Failure {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Dns => formatter.write_str("DNS query failed"),
            Self::Connect => formatter.write_str("failed to connect"),
            Self::ConnectionFailed => formatter.write_str("the connection failed"),
        }
    }
}

impl core::error::Error for Failure {}

#[derive(Debug)]
pub(super) enum State {
    Available,
    Connecting(Request, Protocol),
    Connected,
    Failure(Failure),
}

#[derive(Debug)]
pub(super) struct Socket {
    id: Id,
    frame: u8,
}

impl Socket {
    pub(super) fn new() -> Self {
        Self {
            id: Id::P2P,
            frame: 0,
        }
    }

    pub(super) fn id(&self) -> Id {
        self.id
    }

    pub(super) fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    pub(super) fn frame(&self) -> u8 {
        self.frame
    }

    pub(super) fn increment_frame(&mut self) {
        self.frame = self.frame.saturating_add(1);
    }

    pub(super) fn reset_frame(&mut self) {
        self.frame = 0;
    }
}
