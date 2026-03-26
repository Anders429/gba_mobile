use crate::ArrayVec;
use core::{
    fmt::{self, Display, Formatter},
    net::SocketAddrV4,
};

#[derive(Debug)]
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
}

impl Display for Failure {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Dns => formatter.write_str("DNS query failed"),
            Self::Connect => formatter.write_str("failed to connect"),
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
}

impl Socket {
    pub(super) fn new() -> Self {
        Self { id: Id::P2P }
    }

    pub(super) fn set_id(&mut self, id: Id) {
        self.id = id;
    }
}
