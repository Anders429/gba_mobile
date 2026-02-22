use crate::{
    Driver, Generation,
    link::{Error, Link},
};

#[derive(Debug)]
pub struct Pending {
    pub(crate) generation: Generation,
}

impl Pending {
    pub fn status(&self, driver: &Driver) -> Result<Option<Link>, Error> {
        driver
            .linking_status(self.generation)
            .map(|finished| {
                finished.then(|| Link {
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
