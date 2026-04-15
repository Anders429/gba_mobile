pub mod error;

pub use error::Error;

use crate::{
    Adapter, ArrayVec, Config, Connection, Driver, Generation, Internet, Pending, Socket, config,
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

impl<Socket1, Socket2, Dns, Config> Link<Driver<Socket1, Socket2, Dns, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub fn new(
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Pending<Self, Socket1, Socket2, Dns, Config> {
        Pending::new(Self {
            link_generation: driver.link(),
            driver: PhantomData,
        })
    }

    pub fn close(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Error<Socket1, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .close_link()
            .map_err(Into::into)
    }

    pub fn login<PhoneNumber, Id, Password>(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
        phone_number: PhoneNumber,
        id: Id,
        password: Password,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Result<
        Pending<Internet<Driver<Socket1, Socket2, Dns, Config>>, Socket1, Socket2, Dns, Config>,
        error::login::Error<Socket1, Socket2, Dns, Config>,
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
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<Adapter, Error<Socket1, Socket2, Dns, Config>> {
        driver
            .as_active(self.link_generation)?
            .adapter()
            .map_err(Into::into)
    }
}

impl<Buffer, Socket2, Dns, Config> Link<Driver<Socket<Buffer>, Socket2, Dns, Config>>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub fn accept(
        &self,
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
    ) -> Result<
        Pending<
            Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, connection::P2p>,
            Socket<Buffer>,
            Socket2,
            Dns,
            Config,
        >,
        Error<Socket<Buffer>, Socket2, Dns, Config>,
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
        driver: &mut Driver<Socket<Buffer>, Socket2, Dns, Config>,
        phone_number: PhoneNumber,
    ) -> Result<
        Pending<
            Connection<Driver<Socket<Buffer>, Socket2, Dns, Config>, connection::P2p>,
            Socket<Buffer>,
            Socket2,
            Dns,
            Config,
        >,
        error::connect::Error<Socket<Buffer>, Socket2, Dns, Config>,
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

impl<Socket1, Socket2, Dns, Format> Link<Driver<Socket1, Socket2, Dns, Config<Format>>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Format: config::Format,
{
    pub fn config(
        &self,
        driver: &Driver<Socket1, Socket2, Dns, Config<Format>>,
    ) -> Result<Result<Format, Format::Error>, Error<Socket1, Socket2, Dns, Config<Format>>> {
        driver
            .as_active(self.link_generation)?
            .config()
            .map_err(Into::into)
    }

    pub fn write_config(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config<Format>>,
        format: Format,
    ) -> Result<(), Error<Socket1, Socket2, Dns, Config<Format>>> {
        driver
            .as_active_mut(self.link_generation)?
            .write_config(format)
            .map_err(Into::into)
    }
}

impl<Socket1, Socket2, Dns, Config> PendableError<Socket1, Socket2, Dns, Config>
    for Link<Driver<Socket1, Socket2, Dns, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type Error = Error<Socket1, Socket2, Dns, Config>;
}

impl<Socket1, Socket2, Dns, Config> pending::Sealed<Socket1, Socket2, Dns, Config>
    for Link<Driver<Socket1, Socket2, Dns, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
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
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Self::Error> {
        driver
            .as_active_mut(state.link_generation)?
            .close_link()
            .map_err(Into::into)
    }
}

impl<Socket1, Socket2, Dns, Config> Pendable<Socket1, Socket2, Dns, Config>
    for Link<Driver<Socket1, Socket2, Dns, Config>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}
