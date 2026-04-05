pub mod error;

mod pending;

pub use pending::Pending;

use crate::{Driver, Generation, Socket, dns, socket};
use core::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub struct P2p;

#[derive(Clone, Copy, Debug)]
pub struct Socket1(pub(crate) Generation);

#[derive(Clone, Copy, Debug)]
pub struct Socket2(pub(crate) Generation);

#[derive(Debug)]
pub struct Connection<Driver, Socket> {
    link_generation: Generation,
    connection_generation: Generation,
    socket: Socket,
    driver: PhantomData<Driver>,
}

impl<Buffer, Socket2, Dns> Connection<Driver<Socket<Buffer>, Socket2, Dns>, P2p>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub fn read(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
        buf: &mut [u8],
    ) -> Result<usize, error::io::P2p<Buffer::ReadError, Socket<Buffer>, Socket2, Dns>> {
        driver
            .connection_read(self.link_generation, self.connection_generation, buf)
            .map_err(Into::into)
    }

    pub fn write(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
        buf: &[u8],
    ) -> Result<usize, error::P2p<Socket<Buffer>, Socket2, Dns>> {
        driver
            .connection_write(self.link_generation, self.connection_generation, buf)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns> Connection<Driver<Socket<Buffer>, Socket2, Dns>, Socket1>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub fn read(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
        buf: &mut [u8],
    ) -> Result<usize, error::io::Socket<Buffer::ReadError, Socket<Buffer>, Socket2, Dns>> {
        driver
            .socket_1_read(
                self.link_generation,
                self.connection_generation,
                self.socket.0,
                buf,
            )
            .map_err(Into::into)
    }

    pub fn write(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
        buf: &[u8],
    ) -> Result<usize, error::Socket<Socket<Buffer>, Socket2, Dns>> {
        driver
            .socket_1_write(
                self.link_generation,
                self.connection_generation,
                self.socket.0,
                buf,
            )
            .map_err(Into::into)
    }
}

impl<Buffer, Socket1, Dns> Connection<Driver<Socket1, Socket<Buffer>, Dns>, Socket2>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
{
    pub fn read(
        &mut self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns>,
        buf: &mut [u8],
    ) -> Result<usize, error::io::Socket<Buffer::ReadError, Socket1, Socket<Buffer>, Dns>> {
        driver
            .socket_2_read(
                self.link_generation,
                self.connection_generation,
                self.socket.0,
                buf,
            )
            .map_err(Into::into)
    }

    pub fn write(
        &mut self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns>,
        buf: &[u8],
    ) -> Result<usize, error::Socket<Socket1, Socket<Buffer>, Dns>> {
        driver
            .socket_2_write(
                self.link_generation,
                self.connection_generation,
                self.socket.0,
                buf,
            )
            .map_err(Into::into)
    }
}
