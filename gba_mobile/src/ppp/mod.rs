mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{
    Driver, Generation,
    arrayvec::ArrayVec,
    socket::{self, ToSocket, to_socket::Host},
};
use either::Either;

// TODO: All of this is very similar to P2P connections. Consider combining with a generic.

#[derive(Debug)]
pub struct PPP {
    link_generation: Generation,
    connection_generation: Generation,
}

impl PPP {
    pub fn open_tcp<ToSocket>(
        &self,
        driver: &mut Driver,
        to_socket: ToSocket,
    ) -> Result<socket::Pending, error::socket::Error<ToSocket::Error>>
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
            .open_tcp(
                self.link_generation,
                self.connection_generation,
                internal_host,
                port,
            )?
            .map(|(socket_generation, index)| socket::Pending {
                link_generation: self.link_generation,
                connection_generation: self.connection_generation,
                socket_generation,
                index,
            })
            .ok_or_else(error::socket::Error::no_available_sockets)
    }

    pub fn open_udp<ToSocket>(
        &self,
        driver: &mut Driver,
        to_socket: ToSocket,
    ) -> Result<socket::Pending, error::socket::Error<ToSocket::Error>>
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
            .open_udp(
                self.link_generation,
                self.connection_generation,
                internal_host,
                port,
            )?
            .map(|(socket_generation, index)| socket::Pending {
                link_generation: self.link_generation,
                connection_generation: self.connection_generation,
                socket_generation,
                index,
            })
            .ok_or_else(error::socket::Error::no_available_sockets)
    }
}
