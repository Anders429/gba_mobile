pub mod error;

mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::{
    Adapter, ArrayVec, Config, Driver, Generation, Socket, connection, digit::IntoDigits, dns,
    internet, socket,
};
use core::{marker::PhantomData, net::Ipv4Addr};

#[derive(Debug)]
pub struct Link<Driver> {
    link_generation: Generation,
    driver: PhantomData<Driver>,
}

impl<Socket1, Socket2, Dns> Link<Driver<Socket1, Socket2, Dns>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub fn new(
        driver: &mut Driver<Socket1, Socket2, Dns>,
    ) -> Pending<Driver<Socket1, Socket2, Dns>> {
        Pending {
            link_generation: driver.link(),
            driver: PhantomData,
        }
    }

    pub fn close(self) {
        // TODO
    }

    pub fn login<PhoneNumber, Id, Password>(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns>,
        phone_number: PhoneNumber,
        id: Id,
        password: Password,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Result<internet::Pending<Driver<Socket1, Socket2, Dns>>, error::login::Error>
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
            .map(|connection_generation| internet::Pending {
                link_generation: self.link_generation,
                connection_generation,
                driver: PhantomData,
            })
    }

    pub fn adapter(&self, driver: &Driver<Socket1, Socket2, Dns>) -> Result<Adapter, Error> {
        driver.adapter(self.link_generation).map_err(Into::into)
    }

    pub fn config<Config>(
        &self,
        driver: &Driver<Socket1, Socket2, Dns>,
    ) -> Result<Config, error::config::Error<Config::Error>>
    where
        Config: self::Config,
    {
        driver
            .config(self.link_generation)
            .map_err(Into::into)
            .and_then(|bytes| Config::read(bytes).map_err(error::config::Error::config_error))
    }

    pub fn write_config<Config>(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns>,
        config: Config,
    ) -> Result<(), Error>
    where
        Config: self::Config,
    {
        driver
            .write_config(self.link_generation, config)
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns> Link<Driver<Socket<Buffer>, Socket2, Dns>>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub fn accept(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
    ) -> Result<connection::Pending<Driver<Socket<Buffer>, Socket2, Dns>, connection::P2p>, Error>
    {
        driver
            .accept(self.link_generation)
            .map(|connection_generation| connection::Pending {
                link_generation: self.link_generation,
                connection_generation,
                socket: connection::P2p,
                driver: PhantomData,
            })
            .map_err(Into::into)
    }

    pub fn connect<PhoneNumber>(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
        phone_number: PhoneNumber,
    ) -> Result<
        connection::Pending<Driver<Socket<Buffer>, Socket2, Dns>, connection::P2p>,
        error::connect::Error,
    >
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
            .map(|connection_generation| connection::Pending {
                link_generation: self.link_generation,
                connection_generation,
                socket: connection::P2p,
                driver: PhantomData,
            })
    }
}
