use super::{Dns, Error};
use crate::{
    Driver, Generation,
    pending::{self, Pendable, PendableError},
    socket,
};
use core::net::Ipv4Addr;

#[derive(Debug)]
pub(crate) struct Pending {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) dns_generation: Generation,
}

impl<Socket1, Socket2, const MAX_LEN: usize> PendableError<Socket1, Socket2, Dns<MAX_LEN>>
    for Ipv4Addr
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    type Error = Error<Socket1, Socket2, Dns<MAX_LEN>>;
}

impl<Socket1, Socket2, const MAX_LEN: usize> pending::Sealed<Socket1, Socket2, Dns<MAX_LEN>>
    for Ipv4Addr
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    type State = Pending;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket2, Dns<MAX_LEN>>,
    ) -> Option<Result<Self, Self::Error>> {
        driver
            .as_active(state.link_generation)
            .map_err(Into::into)
            .and_then(|active| active.dns_status(state.connection_generation, state.dns_generation))
            .map_err(Into::into)
            .transpose()
    }

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket1, Socket2, Dns<MAX_LEN>>,
    ) -> Result<(), Self::Error> {
        todo!("cancel the DNS request")
    }
}

impl<Socket1, Socket2, const MAX_LEN: usize> Pendable<Socket1, Socket2, Dns<MAX_LEN>> for Ipv4Addr
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
}
