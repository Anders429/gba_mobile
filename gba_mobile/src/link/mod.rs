pub mod error;

pub use error::Error;

use crate::{
    Adapter, ArrayVec, Config, Connection, Driver, Generation, Internet, Pending, Socket,
    connection,
    digit::IntoDigits,
    dns,
    pending::{self, Pendable, PendableError},
    socket,
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
    pub fn new(driver: &mut Driver<Socket1, Socket2, Dns>) -> Pending<Self, Socket1, Socket2, Dns> {
        Pending::new(Self {
            link_generation: driver.link(),
            driver: PhantomData,
        })
    }

    pub fn close(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns>,
    ) -> Result<(), Error<Socket1, Socket2, Dns>> {
        driver
            .as_active_mut(self.link_generation)?
            .close_link()
            .map_err(Into::into)
    }

    pub fn login<PhoneNumber, Id, Password>(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns>,
        phone_number: PhoneNumber,
        id: Id,
        password: Password,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Result<
        Pending<Internet<Driver<Socket1, Socket2, Dns>>, Socket1, Socket2, Dns>,
        error::login::Error<Socket1, Socket2, Dns>,
    >
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
                                    .as_active_mut(self.link_generation)?
                                    .login(digits, id, password, primary_dns, secondary_dns)
                                    .map_err(Into::into)
                            })
                    })
            })
            .map(|connection_generation| {
                Pending::new(Internet {
                    link_generation: self.link_generation,
                    connection_generation,
                    driver: PhantomData,
                })
            })
    }

    pub fn adapter(
        &self,
        driver: &Driver<Socket1, Socket2, Dns>,
    ) -> Result<Adapter, Error<Socket1, Socket2, Dns>> {
        driver
            .as_active(self.link_generation)?
            .adapter()
            .map_err(Into::into)
    }

    pub fn config<Config>(
        &self,
        driver: &Driver<Socket1, Socket2, Dns>,
    ) -> Result<Config, error::config::Error<Config::Error, Socket1, Socket2, Dns>>
    where
        Config: self::Config,
    {
        driver
            .as_active(self.link_generation)?
            .config()
            .map_err(Into::into)
            .and_then(|bytes| Config::read(bytes).map_err(error::config::Error::config_error))
    }

    pub fn write_config<Config>(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns>,
        config: Config,
    ) -> Result<(), Error<Socket1, Socket2, Dns>>
    where
        Config: self::Config,
    {
        driver
            .as_active_mut(self.link_generation)?
            .write_config(config)
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
    ) -> Result<
        Pending<
            Connection<Driver<Socket<Buffer>, Socket2, Dns>, connection::P2p>,
            Socket<Buffer>,
            Socket2,
            Dns,
        >,
        Error<Socket<Buffer>, Socket2, Dns>,
    > {
        driver
            .as_active_mut(self.link_generation)?
            .accept()
            .map(|connection_generation| {
                Pending::new(Connection {
                    link_generation: self.link_generation,
                    connection_generation,
                    socket: connection::P2p,
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }

    pub fn connect<PhoneNumber>(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns>,
        phone_number: PhoneNumber,
    ) -> Result<
        Pending<
            Connection<Driver<Socket<Buffer>, Socket2, Dns>, connection::P2p>,
            Socket<Buffer>,
            Socket2,
            Dns,
        >,
        error::connect::Error<Socket<Buffer>, Socket2, Dns>,
    >
    where
        PhoneNumber: IntoDigits,
    {
        ArrayVec::try_from_iter(phone_number.into_digits())
            .map_err(Into::into)
            .and_then(|digits| {
                driver
                    .as_active_mut(self.link_generation)?
                    .connect(digits)
                    .map_err(Into::into)
            })
            .map(|connection_generation| {
                Pending::new(Connection {
                    link_generation: self.link_generation,
                    connection_generation,
                    socket: connection::P2p,
                    driver: PhantomData,
                })
            })
    }
}

impl<Socket1, Socket2, Dns> PendableError<Socket1, Socket2, Dns>
    for Link<Driver<Socket1, Socket2, Dns>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type Error = Error<Socket1, Socket2, Dns>;
}

impl<Socket1, Socket2, Dns> pending::Sealed<Socket1, Socket2, Dns>
    for Link<Driver<Socket1, Socket2, Dns>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket2, Dns>,
    ) -> Option<Result<Self, Self::Error>> {
        driver
            .as_active(state.link_generation)
            .and_then(|active| {
                active.link_status().map(|finished| {
                    finished.then(|| Link {
                        link_generation: state.link_generation,
                        driver: PhantomData,
                    })
                })
            })
            .map_err(Into::into)
            .transpose()
    }

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket1, Socket2, Dns>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .close_link()
            .map_err(Into::into)
    }
}

impl<Socket1, Socket2, Dns> Pendable<Socket1, Socket2, Dns> for Link<Driver<Socket1, Socket2, Dns>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
}
