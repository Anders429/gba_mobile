use super::{Connection, error};
use crate::{
    Driver, Socket, dns,
    pending::{self, PendableError},
    socket,
};
use core::marker::PhantomData;

impl<Buffer, Socket2, Dns> PendableError<Socket<Buffer>, Socket2, Dns>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns>, super::P2p>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type Error = error::P2p<Socket<Buffer>, Socket2, Dns>;
}

impl<Buffer, Socket2, Dns> pending::Sealed<Socket<Buffer>, Socket2, Dns>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns>, super::P2p>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket<Buffer>, Socket2, Dns>,
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
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .disconnect(state.connection_generation)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns> PendableError<Socket<Buffer>, Socket2, Dns>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns>, super::Socket1>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type Error = error::Socket<Socket<Buffer>, Socket2, Dns>;
}

impl<Buffer, Socket2, Dns> pending::Sealed<Socket<Buffer>, Socket2, Dns>
    for Connection<Driver<Socket<Buffer>, Socket2, Dns>, super::Socket1>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket<Buffer>, Socket2, Dns>,
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
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .close_socket_1(state.connection_generation, state.socket.0)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket1, Dns> PendableError<Socket1, Socket<Buffer>, Dns>
    for Connection<Driver<Socket1, Socket<Buffer>, Dns>, super::Socket2>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
{
    type Error = error::Socket<Socket1, Socket<Buffer>, Dns>;
}

impl<Buffer, Socket1, Dns> pending::Sealed<Socket1, Socket<Buffer>, Dns>
    for Connection<Driver<Socket1, Socket<Buffer>, Dns>, super::Socket2>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket<Buffer>, Dns>,
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
        driver: &mut Driver<Socket1, Socket<Buffer>, Dns>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .close_socket_2(state.connection_generation, state.socket.0)
            .map_err(Into::into)
    }
}
