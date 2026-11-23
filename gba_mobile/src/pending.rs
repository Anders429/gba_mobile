use crate::{Driver, config, dns, socket};
use core::marker::PhantomData;

pub trait PendableError<Socket1, Socket2, Dns, Config>: Sized {
    type Error;
}

pub(crate) trait Sealed<Socket1, Socket2, Dns, Config>:
    PendableError<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    type State;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Option<Result<Self, Self::Error>>;

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Self::Error>;
}

#[allow(private_bounds)]
pub trait Pendable<Socket1, Socket2, Dns, Config>:
    Sealed<Socket1, Socket2, Dns, Config> + PendableError<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
}

/// A pending result for an asynchronous [`Driver`] operation.
///
/// Due to the nature of packet communication requiring multiple serial reads/writes (and many
/// operations requiring multiple packets to be sent/received), operations cannot be completed
/// immediately. In order to not block on waiting for the response, a `Pending` wrapper is
/// returned, providing a handle to poll the resource.
#[derive(Debug)]
pub struct Pending<T, Socket1, Socket2, Dns, Config>
where
    T: Pendable<Socket1, Socket2, Dns, Config>,
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    state: T::State,
    sockets: PhantomData<(Socket1, Socket2)>,
    dns: PhantomData<Dns>,
    config: PhantomData<Config>,
}

impl<T, Socket1, Socket2, Dns, Config> Pending<T, Socket1, Socket2, Dns, Config>
where
    T: Pendable<Socket1, Socket2, Dns, Config>,
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub(crate) fn new(state: T::State) -> Self {
        Self {
            state,
            sockets: PhantomData,
            dns: PhantomData,
            config: PhantomData,
        }
    }

    /// Poll the pending result's current status.
    ///
    /// Will return `Some(Ok(T))` if the operation was successful, `Some(Err(T::Error))` if the
    /// operation failed, or `None` if the operation is still pending.
    ///
    /// Note that calling this function does not consume `self`. You may call `status()` multiple
    /// times for the same pending operation. However, the result is not guaranteed to always be
    /// available and may be superseded by future operations later.
    pub fn status(
        &self,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Option<Result<T, T::Error>> {
        T::status(&self.state, driver)
    }

    /// Cancel the pending operation.
    ///
    /// If the operation is currently executing, its execution will still finish. In those cases,
    /// the state will be reset and any further necessary communication to undo the operation will
    /// be executed.
    pub fn cancel(
        self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), T::Error> {
        T::cancel(self.state, driver)
    }
}
