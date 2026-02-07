use crate::{
    Driver, Generation,
    link_p2p::{Error, LinkP2P},
};

#[derive(Debug)]
pub struct Pending {
    pub(crate) generation: Generation,
}

impl Pending {
    pub fn status(&self, driver: &Driver) -> Result<Option<LinkP2P>, Error> {
        driver
            .link_p2p_status(self.generation)
            .map(|finished| {
                finished.then(|| LinkP2P {
                    generation: self.generation,
                })
            })
            .map_err(|error| error.into())
    }

    /// Cancel this pending link.
    pub fn cancel(self, driver: &mut Driver) {
        driver.end_session(self.generation);
    }
}
