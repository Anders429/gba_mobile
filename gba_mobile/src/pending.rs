use crate::{Driver, dns, socket};
use core::marker::PhantomData;

// May need to provide generics here somehow.
//
// Otherwise, it can't be used for DNS.
pub trait PendableError<Socket1, Socket2, Dns>: Sized {
    type Error;
}

pub(crate) trait Sealed<Socket1, Socket2, Dns>:
    PendableError<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type State;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket2, Dns>,
    ) -> Option<Result<Self, Self::Error>>;

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket1, Socket2, Dns>,
    ) -> Result<(), Self::Error>;
}

#[allow(private_bounds)]
pub trait Pendable<Socket1, Socket2, Dns>:
    Sealed<Socket1, Socket2, Dns> + PendableError<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
}

impl<T, Socket1, Socket2, Dns> Pendable<Socket1, Socket2, Dns> for T
where
    T: Sealed<Socket1, Socket2, Dns>,
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
}

#[derive(Debug)]
pub struct Pending<T, Socket1, Socket2, Dns>
where
    T: Pendable<Socket1, Socket2, Dns>,
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    state: T::State,
    sockets: PhantomData<(Socket1, Socket2)>,
    dns: PhantomData<Dns>,
}

impl<T, Socket1, Socket2, Dns> Pending<T, Socket1, Socket2, Dns>
where
    T: Pendable<Socket1, Socket2, Dns>,
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn new(state: T::State) -> Self {
        Self {
            state,
            sockets: PhantomData,
            dns: PhantomData,
        }
    }

    pub fn status(&self, driver: &Driver<Socket1, Socket2, Dns>) -> Option<Result<T, T::Error>> {
        T::status(&self.state, driver)
    }

    pub fn cancel(self, driver: &mut Driver<Socket1, Socket2, Dns>) -> Result<(), T::Error> {
        T::cancel(self.state, driver)
    }
}
