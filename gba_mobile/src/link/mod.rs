mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{Driver, Generation, p2p};

#[derive(Debug)]
pub struct Link {
    generation: Generation,
}

impl Link {
    pub fn disconnect(self, driver: &mut Driver) {
        driver.end_session(self.generation);
    }

    pub fn accept(&mut self, driver: &mut Driver) -> Result<p2p::Pending, Error> {
        let call_generation = driver.wait_for_call(self.generation)?;

        Ok(p2p::Pending {
            generation: self.generation,
            call_generation,
        })
    }
}
