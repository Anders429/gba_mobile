pub mod error;

#[doc(inline)]
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

/// A link with the Mobile Adapter.
///
/// This is a lightweight typestate handle indicating that a link with the adapter has been
/// established. Its existence does not guarantee that a link continues to exist. In order to
/// interact with the adapter through the [`Driver`], you must do so through an existing `Link`.
///
/// Through an open `Link` you can:
/// - **Connect to the internet** using [`login()`] to establish a PPP connection.
/// - **Establish a P2P connection** using [`connect()`] or [`accept()`].
/// - **Interact with the Adapter's on-device configuration** with [`config()`] or [`write_config()`].
///
/// Only one `Link` may be open at a time. If a subsequent `Link` is established, any pre-existing
/// ones will become "superseded" and may no longer be used for sending data to the adapter.
///
/// [`accept()`]: Link::accept()
/// [`config()`]: Link::config()
/// [`connect()`]: Link::connect()
/// [`login()`]: Link::login()
/// [`write_config()`]: Link::write_config()
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
    /// Establish a new `Link` through the given driver.
    ///
    /// Returns a [`Pending`] result, which will eventually resolve into a `Link` if the connection
    /// is established, or an error if the connection fails.
    ///
    /// Note that creating a new `Link` will invalidate any previous `Link`s (as well as any
    /// connections you have open through a previous `Link`). Even iff you are already connected to
    /// the adapter, calling this function is not a no-op. The previous session will be closed and
    /// a new session will be opened.
    pub fn new(
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Pending<Self, Socket1, Socket2, Dns, Config> {
        Pending::new(Self {
            link_generation: driver.link(),
            driver: PhantomData,
        })
    }

    /// Close this `Link` connection with the Mobile Adapter.
    pub fn close(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Error<Socket1, Socket2, Dns, Config>> {
        driver
            .as_active_mut(self.link_generation)?
            .close_link()
            .map_err(Into::into)
    }

    /// Open an internet connection with the given credentials.
    ///
    /// The Mobile Adapter connects using
    /// [Point-to-Point Protocol](https://en.wikipedia.org/wiki/Point-to-Point_Protocol). The
    /// `phone_number` provided here should be a valid number to connect to an ISP. `id` and
    /// `password` are used to sign in to the ISP, if applicable (some implementations do not use
    /// these values at all). Consult the documentation for the Mobile Adapter implementation you
    /// are using for more information on what these values should be.
    ///
    /// Returns a [`Pending`] result, which will eventually resolve into an [`Internet`] struct if
    /// login succeeds, or an error if login fails.
    ///
    /// Calling this function will close any existing open P2P or internet connections.
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

    /// Returns the specific type of Mobile Adapter that is linked to.
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
    /// Accept a new incoming P2P connection.
    ///
    /// Returns a [`Pending`] result that will eventually resolve into a P2P [`Connection`] when an
    /// incoming connection is received, or an error if the connection fails. This `Pending` result
    /// will never timeout; timeouts should therefore be handled by the caller of this function if
    /// desired.
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
                    mode: connection::P2p,
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }

    /// Open a P2P connection to a remote host.
    ///
    /// Returns a [`Pending`] result that will eventually resolve into a P2P [`Connection`] when a
    /// connection is established, or an error if the connection fails.
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
                    mode: connection::P2p,
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
    /// Read the Mobile Adapter's on-device EEPROM data in the config format specified for the
    /// [`Driver`].
    ///
    /// See the [`config`] module for more information on config formats.
    pub fn config(
        &self,
        driver: &Driver<Socket1, Socket2, Dns, Config<Format>>,
    ) -> Result<
        Result<Format, Format::Error>,
        error::config::Error<Socket1, Socket2, Dns, Config<Format>>,
    > {
        driver
            .as_active(self.link_generation)
            .map_err(Into::into)
            .and_then(|active| active.config())
            .map_err(Into::into)
    }

    /// Write the given `format` value to the Mobile Adapter's on-device EEPROM data.
    ///
    /// The update will happen immediately in the [`Driver`]. However, the actual update to the
    /// adapter will happen later. It is always guaranteed to happen before the session is ended
    /// after calling [`Link::close()`].
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
