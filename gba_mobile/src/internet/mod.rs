pub mod error;

mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{
    ArrayVec, Dns, Driver, Generation, Socket, connection, dns,
    socket::{self, ToSocket, to_socket::Host},
};
use core::{marker::PhantomData, net::Ipv4Addr};
use either::Either;

#[derive(Debug)]
pub struct Internet<Driver> {
    link_generation: Generation,
    connection_generation: Generation,
    driver: PhantomData<Driver>,
}

impl<Socket1, Socket2, Dns> Internet<Driver<Socket1, Socket2, Dns>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub fn ip(&self, driver: &Driver<Socket1, Socket2, Dns>) -> Result<Ipv4Addr, Error> {
        driver
            .ip(self.link_generation, self.connection_generation)
            .map_err(Into::into)
    }

    pub fn primary_dns(&self, driver: &Driver<Socket1, Socket2, Dns>) -> Result<Ipv4Addr, Error> {
        driver
            .primary_dns(self.link_generation, self.connection_generation)
            .map_err(Into::into)
    }

    pub fn secondary_dns(&self, driver: &Driver<Socket1, Socket2, Dns>) -> Result<Ipv4Addr, Error> {
        driver
            .secondary_dns(self.link_generation, self.connection_generation)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns> Internet<Driver<Socket<Buffer>, Socket2, Dns>>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub fn socket_1_tcp<ToSocket>(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
        to_socket: ToSocket,
    ) -> Result<
        connection::Pending<Driver<Socket<Buffer>, Socket2, Dns>, connection::Socket1>,
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
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
        to_socket: ToSocket,
    ) -> Result<
        connection::Pending<Driver<Socket<Buffer>, Socket2, Dns>, connection::Socket1>,
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

impl<Buffer, Socket1, Dns> Internet<Driver<Socket1, Socket<Buffer>, Dns>>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
{
    pub fn socket_2_tcp<ToSocket>(
        &self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns>,
        to_socket: ToSocket,
    ) -> Result<
        connection::Pending<Driver<Socket1, Socket<Buffer>, Dns>, connection::Socket2>,
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
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns>,
        to_socket: ToSocket,
    ) -> Result<
        connection::Pending<Driver<Socket1, Socket<Buffer>, Dns>, connection::Socket2>,
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

impl<Socket1, Socket2, const MAX_LEN: usize> Internet<Driver<Socket1, Socket2, Dns<MAX_LEN>>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub fn dns<Name>(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns<MAX_LEN>>,
        name: Name,
    ) -> Result<dns::Pending<Driver<Socket1, Socket2, Dns<MAX_LEN>>>, error::dns::Error<MAX_LEN>>
    where
        Name: dns::ToName,
    {
        driver
            .dns(
                self.link_generation,
                self.connection_generation,
                ArrayVec::try_from_iter(name.to_name().into_iter().copied())?,
            )
            .map(|dns_generation| dns::Pending {
                link_generation: self.link_generation,
                connection_generation: self.connection_generation,
                dns_generation,
                driver: PhantomData,
            })
            .map_err(Into::into)
    }
}
