pub mod error;

mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{Adapter, ArrayVec, Config, Driver, Generation, digit::IntoDigits, p2p, ppp};
use core::net::Ipv4Addr;

#[derive(Debug)]
pub struct Link {
    link_generation: Generation,
}

impl Link {
    pub fn new(driver: &mut Driver) -> Pending {
        Pending {
            link_generation: driver.link(),
        }
    }

    pub fn close(self) {
        // TODO
    }

    pub fn accept(&self, driver: &mut Driver) -> Result<p2p::Pending, Error> {
        driver
            .accept(self.link_generation)
            .map(|connection_generation| p2p::Pending {
                link_generation: self.link_generation,
                connection_generation,
            })
            .map_err(Into::into)
    }

    pub fn connect<PhoneNumber>(
        &self,
        driver: &mut Driver,
        phone_number: PhoneNumber,
    ) -> Result<p2p::Pending, error::connect::Error>
    where
        PhoneNumber: IntoDigits,
    {
        ArrayVec::try_from_iter(phone_number.into_digits())
            .map_err(Into::into)
            .and_then(|digits| {
                driver
                    .connect(self.link_generation, digits)
                    .map_err(Into::into)
            })
            .map(|connection_generation| p2p::Pending {
                link_generation: self.link_generation,
                connection_generation,
            })
    }

    pub fn login<PhoneNumber, Id, Password>(
        &self,
        driver: &mut Driver,
        phone_number: PhoneNumber,
        id: Id,
        password: Password,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Result<ppp::Pending, error::login::Error>
    where
        PhoneNumber: IntoDigits,
        Id: IntoIterator<Item = u8>,
        Password: IntoIterator<Item = u8>,
    {
        ArrayVec::try_from_iter(phone_number.into_digits())
            .map_err(Into::into)
            .and_then(|digits| {
                ArrayVec::try_from_iter(id)
                    .map_err(error::login::Error::id)
                    .and_then(|id| {
                        ArrayVec::try_from_iter(password)
                            .map_err(error::login::Error::password)
                            .and_then(|password| {
                                driver
                                    .login(
                                        self.link_generation,
                                        digits,
                                        id,
                                        password,
                                        primary_dns,
                                        secondary_dns,
                                    )
                                    .map_err(Into::into)
                            })
                    })
            })
            .map(|connection_generation| ppp::Pending {
                link_generation: self.link_generation,
                connection_generation,
            })
    }

    pub fn adapter(&self, driver: &Driver) -> Result<Adapter, Error> {
        driver.adapter(self.link_generation).map_err(Into::into)
    }

    pub fn config<Config>(
        &self,
        driver: &Driver,
    ) -> Result<Config, error::config::Error<Config::Error>>
    where
        Config: self::Config,
    {
        driver
            .config(self.link_generation)
            .map_err(Into::into)
            .and_then(|bytes| Config::read(bytes).map_err(error::config::Error::config_error))
    }

    pub fn write_config<Config>(&self, driver: &mut Driver, config: Config) -> Result<(), Error>
    where
        Config: self::Config,
    {
        driver
            .write_config(self.link_generation, config)
            .map_err(Into::into)
    }
}
