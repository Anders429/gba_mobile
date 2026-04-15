pub mod error;

pub use error::Error;

use crate::{
    ArrayVec, Connection, Dns, Driver, Generation, Pending, Socket, config, connection, dns,
    pending::{self, Pendable, PendableError},
    socket,
};
use core::{
    marker::PhantomData,
    net::{Ipv4Addr, SocketAddrV4},
};

#[derive(Debug)]
pub struct Internet<Driver> {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) driver: PhantomData<Driver>,
}

impl<Socket1, Socket2, Dns, Config> Internet<Driver<Socket1, Socket2, Dns, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub fn ip(
        &self,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<Ipv4Addr, Error<Socket1, Socket2, Dns, Config>> {
        driver
            .as_active(self.link_generation)?
            .ip(self.connection_generation)
            .map_err(Into::into)
    }

    pub fn primary_dns(
        &self,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<Ipv4Addr, Error<Socket1, Socket2, Dns, Config>> {
        driver
            .as_active(self.link_generation)?
            .primary_dns(self.connection_generation)
            .map_err(Into::into)
    }

    pub fn secondary_dns(
        &self,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<Ipv4Addr, Error<Socket1, Socket2, Dns, Config>> {
        driver
            .as_active(self.link_generation)?
            .secondary_dns(self.connection_generation)
            .map_err(Into::into)
    }

    pub fn disconnect(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Error<Socket1, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .disconnect(self.connection_generation)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns, Config> Internet<Driver<Socket<Buffer>, Socket2, Dns, Config>>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub fn socket_1_tcp(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        socket_addr: SocketAddrV4,
    ) -> Result<
        Pending<
            Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, connection::Socket1>,
            Socket<Buffer>,
            Socket2,
            Dns,
            Config,
        >,
        Error<Socket<Buffer>, Socket2, Dns, Config>,
    > {
        driver
            .as_active_mut(self.link_generation)?
            .open_tcp_1(self.connection_generation, socket_addr)
            .map(|socket_generation| {
                Pending::new(Connection {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    socket: connection::Socket1(socket_generation),
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }

    pub fn socket_1_upd(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        socket_addr: SocketAddrV4,
    ) -> Result<
        Pending<
            Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, connection::Socket1>,
            Socket<Buffer>,
            Socket2,
            Dns,
            Config,
        >,
        Error<Socket<Buffer>, Socket2, Dns, Config>,
    > {
        driver
            .as_active_mut(self.link_generation)?
            .open_udp_1(self.connection_generation, socket_addr)
            .map(|socket_generation| {
                Pending::new(Connection {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    socket: connection::Socket1(socket_generation),
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }
}

impl<Buffer, Socket1, Dns, Config> Internet<Driver<Socket1, Socket<Buffer>, Dns, Config>>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub fn socket_2_tcp(
        &self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        socket_addr: SocketAddrV4,
    ) -> Result<
        Pending<
            Connection<Driver<Socket1, Socket<Buffer>, Dns, Config>, connection::Socket2>,
            Socket1,
            Socket<Buffer>,
            Dns,
            Config,
        >,
        Error<Socket1, Socket<Buffer>, Dns, Config>,
    > {
        driver
            .as_active_mut(self.link_generation)?
            .open_tcp_2(self.connection_generation, socket_addr)
            .map(|socket_generation| {
                Pending::new(Connection {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    socket: connection::Socket2(socket_generation),
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }

    pub fn socket_2_upd(
        &self,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
        socket_addr: SocketAddrV4,
    ) -> Result<
        Pending<
            Connection<Driver<Socket1, Socket<Buffer>, Dns, Config>, connection::Socket2>,
            Socket1,
            Socket<Buffer>,
            Dns,
            Config,
        >,
        Error<Socket1, Socket<Buffer>, Dns, Config>,
    > {
        driver
            .as_active_mut(self.link_generation)?
            .open_udp_2(self.connection_generation, socket_addr)
            .map(|socket_generation| {
                Pending::new(Connection {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    socket: connection::Socket2(socket_generation),
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }
}

impl<Socket1, Socket2, Config, const MAX_LEN: usize>
    Internet<Driver<Socket1, Socket2, Dns<MAX_LEN>, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Config: config::Mode,
{
    pub fn dns<Name>(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns<MAX_LEN>, Config>,
        name: Name,
    ) -> Result<
        Pending<Ipv4Addr, Socket1, Socket2, Dns<MAX_LEN>, Config>,
        error::dns::Error<Socket1, Socket2, Dns<MAX_LEN>, Config, MAX_LEN>,
    >
    where
        Name: dns::ToName,
    {
        driver
            .as_active_mut(self.link_generation)?
            .dns(
                self.connection_generation,
                ArrayVec::try_from_iter(name.to_name().into_iter().copied())?,
            )
            .map(|dns_generation| {
                Pending::new(dns::Pending {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    dns_generation,
                })
            })
            .map_err(Into::into)
    }
}

impl<Socket1, Socket2, Dns, Config> PendableError<Socket1, Socket2, Dns, Config>
    for Internet<Driver<Socket1, Socket2, Dns, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type Error = Error<Socket1, Socket2, Dns, Config>;
}

impl<Socket1, Socket2, Dns, Config> pending::Sealed<Socket1, Socket2, Dns, Config>
    for Internet<Driver<Socket1, Socket2, Dns, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Option<Result<Self, Self::Error>> {
        driver
            .as_active(state.link_generation)
            .map_err(Into::into)
            .and_then(|active| {
                active
                    .connection_status(state.connection_generation)
                    .map(|finished| {
                        finished.then(|| Internet {
                            link_generation: state.link_generation,
                            connection_generation: state.connection_generation,
                            driver: PhantomData,
                        })
                    })
            })
            .map_err(Into::into)
            .transpose()
    }

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .disconnect(state.connection_generation)
            .map_err(Into::into)
    }
}

impl<Socket1, Socket2, Dns, Config> Pendable<Socket1, Socket2, Dns, Config>
    for Internet<Driver<Socket1, Socket2, Dns, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}
