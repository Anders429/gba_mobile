use crate::{
    Driver, Generation,
    link::{Error, Link},
};

#[derive(Debug)]
pub struct Pending {
    pub(crate) link_generation: Generation,
}

impl Pending {
    pub fn status(&self, driver: &Driver) -> Result<Option<Link>, Error> {
        driver
            .link_status(self.link_generation)
            .map(|finished| {
                finished.then(|| Link {
                    link_generation: self.link_generation,
                })
            })
            .map_err(|error| error.into())
    }

    /// Cancel this pending link.
    pub fn cancel(self) {
        todo!()
    }
}
