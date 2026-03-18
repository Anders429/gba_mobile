use super::{Error, PPP};
use crate::{DRIVER, Generation, mmio::interrupt};

#[derive(Debug)]
pub struct Pending {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
}

impl Pending {
    pub fn status(&self) -> Result<Option<PPP>, Error> {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let result = DRIVER
                .connection_status(self.link_generation, self.connection_generation)
                .map(|finished| {
                    finished.then(|| PPP {
                        link_generation: self.link_generation,
                        connection_generation: self.connection_generation,
                    })
                })
                .map_err(|error| error.into());
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            result
        }
    }
}
