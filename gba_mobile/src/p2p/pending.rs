use super::{Error, P2P};
use crate::{Driver, Generation};

#[derive(Debug)]
pub struct Pending {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
}

impl Pending {
    pub fn status(&self, driver: &Driver) -> Result<Option<P2P>, Error> {
        driver
            .connection_status(self.link_generation, self.connection_generation)
            .map(|finished| {
                finished.then(|| P2P {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                })
            })
            .map_err(|error| error.into())
    }
}
