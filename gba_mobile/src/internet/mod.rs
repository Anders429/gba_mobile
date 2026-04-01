pub mod error;

mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{
    ArrayVec, Driver, Generation, Socket, connection, socket,
    socket::{ToSocket, to_socket::Host},
};
use core::{marker::PhantomData, net::Ipv4Addr};
use either::Either;
use embedded_io::{Read, Write};

#[derive(Debug)]
pub struct Internet<Driver> {
    link_generation: Generation,
    connection_generation: Generation,
    driver: PhantomData<Driver>,
}

impl<Socket1, Socket2> Internet<Driver<Socket1, Socket2>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub fn ip(&self, driver: &Driver<Socket1, Socket2>) -> Result<Ipv4Addr, Error> {
        driver
            .ip(self.link_generation, self.connection_generation)
            .map_err(Into::into)
    }

    pub fn primary_dns(&self, driver: &Driver<Socket1, Socket2>) -> Result<Ipv4Addr, Error> {
        driver
            .primary_dns(self.link_generation, self.connection_generation)
            .map_err(Into::into)
    }

    pub fn secondary_dns(&self, driver: &Driver<Socket1, Socket2>) -> Result<Ipv4Addr, Error> {
        driver
            .secondary_dns(self.link_generation, self.connection_generation)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2> Internet<Driver<Socket<Buffer>, Socket2>>
where
    Buffer: Read + Write,
    Socket2: socket::Slot,
{
    pub fn socket_1_tcp<ToSocket>(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2>,
        to_socket: ToSocket,
    ) -> Result<
        connection::Pending<Driver<Socket<Buffer>, Socket2>, connection::Socket1>,
        error::socket::Error<ToSocket::Error>,
    >
    where
        ToSocket: self::ToSocket,
    {
        let (host, port) = to_socket
            .to_socket()
            .map_err(error::socket::Error::socket)?;
        let internal_host = match host {
            Host::Ip(ip) => Either::Left(ip),
            Host::Name(name) => Either::Right(ArrayVec::try_from_iter(name.into_iter().copied())?),
        };

        driver
            .open_tcp_1(
                self.link_generation,
                self.connection_generation,
                internal_host,
                port,
            )
            .map(|socket_generation| connection::Pending {
                link_generation: self.link_generation,
                connection_generation: self.connection_generation,
                socket: connection::Socket1(socket_generation),
                driver: PhantomData,
            })
            .map_err(Into::into)
    }

    pub fn socket_1_upd<ToSocket>(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2>,
        to_socket: ToSocket,
    ) -> Result<
        connection::Pending<Driver<Socket<Buffer>, Socket2>, connection::Socket1>,
        error::socket::Error<ToSocket::Error>,
    >
    where
        ToSocket: self::ToSocket,
    {
        let (host, port) = to_socket
            .to_socket()
            .map_err(error::socket::Error::socket)?;
        let internal_host = match host {
            Host::Ip(ip) => Either::Left(ip),
            Host::Name(name) => Either::Right(ArrayVec::try_from_iter(name.into_iter().copied())?),
        };

        driver
            .open_udp_1(
                self.link_generation,
                self.connection_generation,
                internal_host,
                port,
            )
            .map(|socket_generation| connection::Pending {
                link_generation: self.link_generation,
                connection_generation: self.connection_generation,
                socket: connection::Socket1(socket_generation),
                driver: PhantomData,
            })
            .map_err(Into::into)
    }
}

impl<Buffer, Socket1> Internet<Driver<Socket1, Socket<Buffer>>>
where
    Buffer: Read + Write,
    Socket1: socket::Slot,
{
    pub fn socket_2_tcp<ToSocket>(
        &self,
        driver: &mut Driver<Socket1, Socket<Buffer>>,
        to_socket: ToSocket,
    ) -> Result<
        connection::Pending<Driver<Socket1, Socket<Buffer>>, connection::Socket2>,
        error::socket::Error<ToSocket::Error>,
    >
    where
        ToSocket: self::ToSocket,
    {
        let (host, port) = to_socket
            .to_socket()
            .map_err(error::socket::Error::socket)?;
        let internal_host = match host {
            Host::Ip(ip) => Either::Left(ip),
            Host::Name(name) => Either::Right(ArrayVec::try_from_iter(name.into_iter().copied())?),
        };

        driver
            .open_tcp_2(
                self.link_generation,
                self.connection_generation,
                internal_host,
                port,
            )
            .map(|socket_generation| connection::Pending {
                link_generation: self.link_generation,
                connection_generation: self.connection_generation,
                socket: connection::Socket2(socket_generation),
                driver: PhantomData,
            })
            .map_err(Into::into)
    }

    pub fn socket_2_upd<ToSocket>(
        &self,
        driver: &mut Driver<Socket1, Socket<Buffer>>,
        to_socket: ToSocket,
    ) -> Result<
        connection::Pending<Driver<Socket1, Socket<Buffer>>, connection::Socket2>,
        error::socket::Error<ToSocket::Error>,
    >
    where
        ToSocket: self::ToSocket,
    {
        let (host, port) = to_socket
            .to_socket()
            .map_err(error::socket::Error::socket)?;
        let internal_host = match host {
            Host::Ip(ip) => Either::Left(ip),
            Host::Name(name) => Either::Right(ArrayVec::try_from_iter(name.into_iter().copied())?),
        };

        driver
            .open_udp_2(
                self.link_generation,
                self.connection_generation,
                internal_host,
                port,
            )
            .map(|socket_generation| connection::Pending {
                link_generation: self.link_generation,
                connection_generation: self.connection_generation,
                socket: connection::Socket2(socket_generation),
                driver: PhantomData,
            })
            .map_err(Into::into)
    }
}
