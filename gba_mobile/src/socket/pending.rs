use super::{Error, Index, Socket};
use crate::{DRIVER, Generation, mmio::interrupt};

#[derive(Debug)]
pub struct Pending {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) socket_generation: Generation,
    pub(crate) index: Index,
}

impl Pending {
    pub fn status(&self) -> Result<Option<Socket>, Error> {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let result = DRIVER
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
                .map_err(|error| error.into());
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            result
        }
    }
}
