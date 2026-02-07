mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{Driver, Generation};

#[derive(Debug)]
pub struct LinkP2P {
    generation: Generation,
}

impl LinkP2P {
    pub fn disconnect(self, driver: &mut Driver) {
        driver.end_session(self.generation);
    }
}
