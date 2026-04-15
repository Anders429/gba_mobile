use super::{Connection, error};
use crate::{
    Driver, Socket, config, dns,
    pending::{self, Pendable, PendableError},
    socket,
};
use core::marker::PhantomData;

impl<Buffer, Socket2, Dns, Config> PendableError<Socket<Buffer>, Socket2, Dns, Config>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, super::P2p>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type Error = error::P2p<Socket<Buffer>, Socket2, Dns, Config>;
}

impl<Buffer, Socket2, Dns, Config> pending::Sealed<Socket<Buffer>, Socket2, Dns, Config>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, super::P2p>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Option<Result<Self, Self::Error>> {
        driver
            .as_active(state.link_generation)
            .map_err(Into::into)
            .and_then(|active| {
                active
                    .connection_status(state.connection_generation)
                    .map(|finished| {
                        finished.then(|| Connection {
                            link_generation: state.link_generation,
                            connection_generation: state.connection_generation,
                            socket: state.socket,
                            driver: PhantomData,
                        })
                    })
            })
            .map_err(Into::into)
            .transpose()
    }

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .disconnect(state.connection_generation)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns, Config> Pendable<Socket<Buffer>, Socket2, Dns, Config>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, super::P2p>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}

impl<Buffer, Socket2, Dns, Config> PendableError<Socket<Buffer>, Socket2, Dns, Config>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, super::Socket1>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type Error = error::Socket<Socket<Buffer>, Socket2, Dns, Config>;
}

impl<Buffer, Socket2, Dns, Config> pending::Sealed<Socket<Buffer>, Socket2, Dns, Config>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, super::Socket1>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Option<Result<Self, Self::Error>> {
        driver
            .as_active(state.link_generation)
            .map_err(Into::into)
            .and_then(|active| {
                active
                    .socket_1_status(state.connection_generation, state.socket.0)
                    .map(|finished| {
                        finished.then(|| Connection {
                            link_generation: state.link_generation,
                            connection_generation: state.connection_generation,
                            socket: state.socket,
                            driver: PhantomData,
                        })
                    })
            })
            .map_err(Into::into)
            .transpose()
    }

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .close_socket_1(state.connection_generation, state.socket.0)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns, Config> Pendable<Socket<Buffer>, Socket2, Dns, Config>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, super::Socket1>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}

impl<Buffer, Socket1, Dns, Config> PendableError<Socket1, Socket<Buffer>, Dns, Config>
    for Connection<Driver<Socket1, Socket<Buffer>, Dns, Config>, super::Socket2>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type Error = error::Socket<Socket1, Socket<Buffer>, Dns, Config>;
}

impl<Buffer, Socket1, Dns, Config> pending::Sealed<Socket1, Socket<Buffer>, Dns, Config>
    for Connection<Driver<Socket1, Socket<Buffer>, Dns, Config>, super::Socket2>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket<Buffer>, Dns, Config>,
    ) -> Option<Result<Self, Self::Error>> {
        driver
            .as_active(state.link_generation)
            .map_err(Into::into)
            .and_then(|active| {
                active
                    .socket_2_status(state.connection_generation, state.socket.0)
                    .map(|finished| {
                        finished.then(|| Connection {
                            link_generation: state.link_generation,
                            connection_generation: state.connection_generation,
                            socket: state.socket,
                            driver: PhantomData,
                        })
                    })
            })
            .map_err(Into::into)
            .transpose()
    }

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns, Config>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .close_socket_2(state.connection_generation, state.socket.0)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket1, Dns, Config> Pendable<Socket1, Socket<Buffer>, Dns, Config>
    for Connection<Driver<Socket1, Socket<Buffer>, Dns, Config>, super::Socket2>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}
