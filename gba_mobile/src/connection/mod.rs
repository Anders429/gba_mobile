pub mod error;

mod pending;

use crate::{Driver, Generation, Socket, config, dns, socket};
use core::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub struct P2p;

#[derive(Clone, Copy, Debug)]
pub struct Socket1(pub(crate) Generation);

#[derive(Clone, Copy, Debug)]
pub struct Socket2(pub(crate) Generation);

#[derive(Debug)]
pub struct Connection<Driver, Socket> {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) socket: Socket,
    pub(crate) driver: PhantomData<Driver>,
}

impl<Buffer, Socket2, Dns, Config> Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, P2p>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub fn read(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        buf: &mut [u8],
    ) -> Result<usize, error::io::P2p<Buffer::ReadError, Socket<Buffer>, Socket2, Dns, Config>>
    {
        driver
            .as_active_mut(self.link_generation)?
            .connection_read(self.connection_generation, buf)
            .map_err(Into::into)
    }

    pub fn write(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        buf: &[u8],
    ) -> Result<usize, error::P2p<Socket<Buffer>, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .connection_write(self.connection_generation, buf)
            .map_err(Into::into)
    }

    pub fn flush(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Result<(), error::P2p<Socket<Buffer>, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .connection_flush(self.connection_generation)
            .map_err(Into::into)
    }

    pub fn close(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Result<(), error::P2p<Socket<Buffer>, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .disconnect(self.connection_generation)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns, Config> Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, Socket1>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub fn read(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        buf: &mut [u8],
    ) -> Result<usize, error::io::Socket<Buffer::ReadError, Socket<Buffer>, Socket2, Dns, Config>>
    {
        driver
            .as_active_mut(self.link_generation)?
            .socket_1_read(self.connection_generation, self.socket.0, buf)
            .map_err(Into::into)
    }

    pub fn write(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        buf: &[u8],
    ) -> Result<usize, error::Socket<Socket<Buffer>, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .socket_1_write(self.connection_generation, self.socket.0, buf)
            .map_err(Into::into)
    }

    pub fn flush(
        &mut self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Result<(), error::Socket<Socket<Buffer>, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .socket_1_flush(self.connection_generation, self.socket.0)
            .map_err(Into::into)
    }

    pub fn close(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Result<(), error::Socket<Socket<Buffer>, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .close_socket_1(self.connection_generation, self.socket.0)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket1, Dns, Config> Connection<Driver<Socket1, Socket<Buffer>, Dns, Config>, Socket2>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub fn read(
        &mut self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        buf: &mut [u8],
    ) -> Result<usize, error::io::Socket<Buffer::ReadError, Socket1, Socket<Buffer>, Dns, Config>>
    {
        driver
            .as_active_mut(self.link_generation)?
            .socket_2_read(self.connection_generation, self.socket.0, buf)
            .map_err(Into::into)
    }

    pub fn write(
        &mut self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        buf: &[u8],
    ) -> Result<usize, error::Socket<Socket1, Socket<Buffer>, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .socket_2_write(self.connection_generation, self.socket.0, buf)
            .map_err(Into::into)
    }

    pub fn flush(
        &mut self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
    ) -> Result<(), error::Socket<Socket1, Socket<Buffer>, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .socket_2_flush(self.connection_generation, self.socket.0)
            .map_err(Into::into)
    }

    pub fn close(
        &self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
    ) -> Result<(), error::Socket<Socket1, Socket<Buffer>, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .close_socket_2(self.connection_generation, self.socket.0)
            .map_err(Into::into)
    }
}
