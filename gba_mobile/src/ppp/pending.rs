use super::{Error, PPP};
use crate::{Driver, Generation};

#[derive(Debug)]
pub struct Pending {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
}

impl Pending {
    pub fn status(&self, driver: &mut Driver) -> Result<Option<PPP>, Error> {
        driver
            .connection_status(self.link_generation, self.connection_generation)
            .map(|finished| {
                finished.then(|| PPP {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                })
            })
            .map_err(|error| error.into())
    }
}
