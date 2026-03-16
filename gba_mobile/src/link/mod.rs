mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{
    Adapter, ArrayVec, Config, DRIVER, Generation, Timer, digit::IntoDigits, mmio::interrupt, p2p,
};

#[derive(Debug)]
pub struct Link {
    link_generation: Generation,
}

impl Link {
    pub fn new(timer: Timer) -> Pending {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let link_generation = DRIVER.link(timer);
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            Pending { link_generation }
        }
    }

    pub fn close(self) {
        // TODO
    }

    pub fn accept(&self) -> Result<p2p::Pending, Error> {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let result = DRIVER.accept(self.link_generation);
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            result
                .map(|connection_generation| p2p::Pending {
                    link_generation: self.link_generation,
                    connection_generation,
                })
                .map_err(Into::into)
        }
    }

    pub fn connect<PhoneNumber>(
        &self,
        phone_number: PhoneNumber,
    ) -> Result<p2p::Pending, error::connect::Error>
    where
        PhoneNumber: IntoDigits,
    {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let connection_generation = DRIVER.connect(
                self.link_generation,
                ArrayVec::try_from_iter(phone_number.into_digits())?,
            )?;
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            Ok(p2p::Pending {
                link_generation: self.link_generation,
                connection_generation,
            })
        }
    }

    pub fn adapter(&self) -> Result<Adapter, Error> {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let result = DRIVER.adapter(self.link_generation);
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            result.map_err(Into::into)
        }
    }

    pub fn config<Config>(&self) -> Result<Config, error::config::Error<Config::Error>>
    where
        Config: self::Config,
    {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let result = Config::read(DRIVER.config(self.link_generation)?)
                .map_err(error::config::Error::config_error);
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            result
        }
    }
}
