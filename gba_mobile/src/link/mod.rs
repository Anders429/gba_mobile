mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{ArrayVec, Driver, Generation, p2p, phone_number::IntoDigits};

#[derive(Debug)]
pub struct Link {
    generation: Generation,
}

impl Link {
    pub fn disconnect(self, driver: &mut Driver) {
        driver.end_session(self.generation);
    }

    pub fn accept(&self, driver: &mut Driver) -> Result<p2p::Pending, Error> {
        let call_generation = driver.wait_for_call(self.generation)?;

        Ok(p2p::Pending {
            generation: self.generation,
            call_generation,
        })
    }

    pub fn connect<PhoneNumber>(
        &self,
        phone_number: PhoneNumber,
        driver: &mut Driver,
    ) -> Result<p2p::Pending, Error>
    where
        PhoneNumber: IntoDigits,
    {
        let call_generation = driver.call(
            ArrayVec::try_from_iter(phone_number.into_digits()).expect("FIX ME"),
            self.generation,
        )?;

        Ok(p2p::Pending {
            generation: self.generation,
            call_generation,
        })
    }
}
