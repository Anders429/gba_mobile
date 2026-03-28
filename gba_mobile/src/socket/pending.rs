use super::{Error, Index, Socket};
use crate::{Driver, Generation};

#[derive(Debug)]
pub struct Pending {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) socket_generation: Generation,
    pub(crate) index: Index,
}

impl Pending {
    pub fn status(&self, driver: &mut Driver) -> Result<Option<Socket>, Error> {
        driver
            .socket_status(
                self.link_generation,
                self.connection_generation,
                self.socket_generation,
                self.index,
            )
            .map(|finished| {
                finished.then(|| Socket {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    socket_generation: self.socket_generation,
                    index: self.index,
                })
            })
            .map_err(|error| error.into())
    }
}
