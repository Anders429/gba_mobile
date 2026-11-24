use crate::{Driver, Generation, Socket, config, connection, dns, socket};

pub trait Error<Socket1, Socket2, Dns, Config> {
    type IoError;
    type Error;
}

pub(crate) trait Sealed<Socket1, Socket2, Dns, Config>:
    Error<Socket1, Socket2, Dns, Config> + Copy
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn read(
        self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, Self::IoError>;

    fn write(
        self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &[u8],
    ) -> Result<usize, Self::Error>;

    fn flush(
        self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error>;

    fn close(
        self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error>;

    fn status(
        self,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<bool, Self::Error>;

    fn cancel(
        self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error>;
}

#[allow(private_bounds)]
pub trait Mode<Socket1, Socket2, Dns, Config>: Sealed<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}

#[derive(Clone, Copy, Debug)]
pub struct P2p;

impl<Buffer, Socket2, Dns, Config> Error<Socket<Buffer>, Socket2, Dns, Config> for P2p
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type IoError =
        connection::error::io::P2p<Buffer::ReadError, Socket<Buffer>, Socket2, Dns, Config>;
    type Error = connection::error::P2p<Socket<Buffer>, Socket2, Dns, Config>;
}

impl<Buffer, Socket2, Dns, Config> Sealed<Socket<Buffer>, Socket2, Dns, Config> for P2p
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn read(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, Self::IoError> {
        driver
            .as_active_mut(link_generation)?
            .connection_read(connection_generation, buf)
            .map_err(Into::into)
    }

    fn write(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &[u8],
    ) -> Result<usize, Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .connection_write(connection_generation, buf)
            .map_err(Into::into)
    }

    fn flush(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .connection_flush(connection_generation)
            .map_err(Into::into)
    }

    fn close(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .disconnect(connection_generation)
            .map_err(Into::into)
    }

    fn status(
        self,
        driver: &Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<bool, Self::Error> {
        driver
            .as_active(link_generation)?
            .connection_status(connection_generation)
            .map_err(Into::into)
    }

    fn cancel(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .disconnect(connection_generation)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns, Config> Mode<Socket<Buffer>, Socket2, Dns, Config> for P2p
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}

#[derive(Clone, Copy, Debug)]
pub struct Socket1(pub(crate) Generation);

impl<Buffer, Socket2, Dns, Config> Error<Socket<Buffer>, Socket2, Dns, Config> for Socket1
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type IoError =
        connection::error::io::Socket<Buffer::ReadError, Socket<Buffer>, Socket2, Dns, Config>;
    type Error = connection::error::Socket<Socket<Buffer>, Socket2, Dns, Config>;
}

impl<Buffer, Socket2, Dns, Config> Sealed<Socket<Buffer>, Socket2, Dns, Config> for Socket1
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn read(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, Self::IoError> {
        driver
            .as_active_mut(link_generation)?
            .socket_1_read(connection_generation, self.0, buf)
            .map_err(Into::into)
    }

    fn write(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &[u8],
    ) -> Result<usize, Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .socket_1_write(connection_generation, self.0, buf)
            .map_err(Into::into)
    }

    fn flush(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .socket_1_flush(connection_generation, self.0)
            .map_err(Into::into)
    }

    fn close(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .close_socket_1(connection_generation, self.0)
            .map_err(Into::into)
    }

    fn status(
        self,
        driver: &Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<bool, Self::Error> {
        driver
            .as_active(link_generation)?
            .socket_1_status(connection_generation, self.0)
            .map_err(Into::into)
    }

    fn cancel(
        self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .close_socket_1(connection_generation, self.0)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns, Config> Mode<Socket<Buffer>, Socket2, Dns, Config> for Socket1
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}

#[derive(Clone, Copy, Debug)]
pub struct Socket2(pub(crate) Generation);

impl<Buffer, Socket1, Dns, Config> Error<Socket1, Socket<Buffer>, Dns, Config> for Socket2
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type IoError =
        connection::error::io::Socket<Buffer::ReadError, Socket1, Socket<Buffer>, Dns, Config>;
    type Error = connection::error::Socket<Socket1, Socket<Buffer>, Dns, Config>;
}

impl<Buffer, Socket1, Dns, Config> Sealed<Socket1, Socket<Buffer>, Dns, Config> for Socket2
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn read(
        self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, Self::IoError> {
        driver
            .as_active_mut(link_generation)?
            .socket_2_read(connection_generation, self.0, buf)
            .map_err(Into::into)
    }

    fn write(
        self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &[u8],
    ) -> Result<usize, Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .socket_2_write(connection_generation, self.0, buf)
            .map_err(Into::into)
    }

    fn flush(
        self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .socket_2_flush(connection_generation, self.0)
            .map_err(Into::into)
    }

    fn close(
        self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .close_socket_2(connection_generation, self.0)
            .map_err(Into::into)
    }

    fn status(
        self,
        driver: &Driver<Socket1, Socket<Buffer>, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<bool, Self::Error> {
        driver
            .as_active(link_generation)?
            .socket_2_status(connection_generation, self.0)
            .map_err(Into::into)
    }

    fn cancel(
        self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(link_generation)?
            .close_socket_2(connection_generation, self.0)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket1, Dns, Config> Mode<Socket1, Socket<Buffer>, Dns, Config> for Socket2
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}
