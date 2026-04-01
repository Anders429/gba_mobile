use super::{Connection, error};
use crate::{Driver, Generation, Socket, socket};
use core::marker::PhantomData;

#[derive(Debug)]
pub struct Pending<Driver, Socket> {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) socket: Socket,
    pub(crate) driver: PhantomData<Driver>,
}

impl<Buffer, Socket2> Pending<Driver<Socket<Buffer>, Socket2>, super::P2p>
where
    Socket2: socket::Slot,
{
    pub fn status(
        &self,
        driver: &Driver<Socket<Buffer>, Socket2>,
    ) -> Result<Option<Connection<Driver<Socket<Buffer>, Socket2>, super::P2p>>, error::P2p> {
        driver
            .connection_status(self.link_generation, self.connection_generation)
            .map(|finished| {
                finished.then(|| Connection {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    socket: self.socket,
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2> Pending<Driver<Socket<Buffer>, Socket2>, super::Socket1>
where
    Socket2: socket::Slot,
{
    pub fn status(
        &self,
        driver: &Driver<Socket<Buffer>, Socket2>,
    ) -> Result<Option<Connection<Driver<Socket<Buffer>, Socket2>, super::Socket1>>, error::Socket>
    {
        driver
            .socket_1_status(
                self.link_generation,
                self.connection_generation,
                self.socket.0,
            )
            .map(|finished| {
                finished.then(|| Connection {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    socket: self.socket,
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }
}

impl<Socket1, Buffer> Pending<Driver<Socket1, Socket<Buffer>>, super::Socket2>
where
    Socket1: socket::Slot,
{
    pub fn status(
        &self,
        driver: &Driver<Socket1, Socket<Buffer>>,
    ) -> Result<Option<Connection<Driver<Socket1, Socket<Buffer>>, super::Socket2>>, error::Socket>
    {
        driver
            .socket_2_status(
                self.link_generation,
                self.connection_generation,
                self.socket.0,
            )
            .map(|finished| {
                finished.then(|| Connection {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    socket: self.socket,
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }
}
