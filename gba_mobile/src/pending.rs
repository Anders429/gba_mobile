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

    pub fn status(
        &self,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Option<Result<T, T::Error>> {
        T::status(&self.state, driver)
    }

    pub fn cancel(
        self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), T::Error> {
        T::cancel(self.state, driver)
    }
}
