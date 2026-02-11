pub mod connection;

mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{Driver, Generation};

#[derive(Debug)]
pub struct Link {
    generation: Generation,
}

impl Link {
    pub fn disconnect(self, driver: &mut Driver) {
        driver.end_session(self.generation);
    }

    pub fn accept(&mut self, driver: &mut Driver) -> Result<connection::Pending, Error> {
        driver.wait_for_call(self.generation)?;

        Ok(connection::Pending {})
    }
}
