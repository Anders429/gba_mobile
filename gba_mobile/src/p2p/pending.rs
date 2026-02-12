use super::{Error, P2P};
use crate::{Driver, Generation};

#[derive(Debug)]
pub struct Pending {
    pub(crate) generation: Generation,
    pub(crate) call_generation: Generation,
}

impl Pending {
    pub fn status(&self, driver: &Driver) -> Result<Option<P2P>, Error> {
        driver
            .p2p_status(self.generation, self.call_generation)
            .map(|finished| {
                finished.then(|| P2P {
                    generation: self.generation,
                    call_generation: self.call_generation,
                })
            })
            .map_err(|error| error.into())
    }
}
