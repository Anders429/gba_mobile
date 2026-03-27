pub mod error;

mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{
    Adapter, ArrayVec, Config, DRIVER, Generation, digit::IntoDigits, mmio::interrupt, p2p, ppp,
};
use core::net::Ipv4Addr;

#[derive(Debug)]
pub struct Link {
    link_generation: Generation,
}

impl Link {
    pub fn new() -> Pending {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let link_generation = DRIVER.link();
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
            let result = ArrayVec::try_from_iter(phone_number.into_digits())
                .map_err(Into::into)
                .and_then(|digits| {
                    DRIVER
                        .connect(self.link_generation, digits)
                        .map_err(Into::into)
                });
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);

            result.map(|connection_generation| p2p::Pending {
                link_generation: self.link_generation,
                connection_generation,
            })
        }
    }

    pub fn login<PhoneNumber, Id, Password>(
        &self,
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
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let result = ArrayVec::try_from_iter(phone_number.into_digits())
                .map_err(Into::into)
                .and_then(|digits| {
                    ArrayVec::try_from_iter(id)
                        .map_err(error::login::Error::id)
                        .and_then(|id| {
                            ArrayVec::try_from_iter(password)
                                .map_err(error::login::Error::password)
                                .and_then(|password| {
                                    DRIVER
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
                });
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);

            result.map(|connection_generation| ppp::Pending {
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
            let result = DRIVER
                .config(self.link_generation)
                .map_err(Into::into)
                .and_then(|bytes| Config::read(bytes).map_err(error::config::Error::config_error));
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            result
        }
    }

    pub fn write_config<Config>(&self, config: Config) -> Result<(), Error>
    where
        Config: self::Config,
    {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let result = DRIVER
                .write_config(self.link_generation, config)
                .map_err(Into::into);
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            result
        }
    }
}
