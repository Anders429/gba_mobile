pub mod to_socket;

pub(crate) mod slot;

mod buffer;

pub use buffer::Buffer;
pub use slot::Slot;
pub use to_socket::ToSocket;

use crate::ArrayVec;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub(crate) struct Id(pub(crate) u8);

impl Id {
    pub(crate) const P2P: Id = Id(0xff);
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
pub(crate) enum Status {
    NotConnected,
    Connecting,
    Connected,
    ConnectionFailure,
    ConnectionLost,
    ClosedRemotely,
}

#[derive(Debug)]
pub struct Socket<Buffer> {
    pub(crate) read_buffer: Buffer,
    pub(crate) write_buffer: ArrayVec<u8, 254>,
    pub(crate) frame: u8,
    pub(crate) id: Id,
    pub(crate) status: Status,
}

impl<Buffer> Socket<Buffer> {
    pub const fn new(buffer: Buffer) -> Self {
        Self {
            read_buffer: buffer,
            write_buffer: ArrayVec::new(),
            frame: 0,
            id: Id::P2P,
            status: Status::NotConnected,
        }
    }
}

impl<Buffer> Socket<Buffer>
where
    Buffer: self::Buffer,
{
    pub(crate) fn read(&mut self, buf: &mut [u8]) -> Result<usize, Buffer::ReadError> {
        self.read_buffer.read(buf)
    }

    pub(crate) fn write(&mut self, buf: &[u8]) -> usize {
        self.write_buffer.write(buf)
    }
}

#[derive(Debug)]
pub struct NoSocket;
