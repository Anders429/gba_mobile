use super::{Dns, Error};
use crate::{Driver, Generation, socket};
use core::{marker::PhantomData, net::Ipv4Addr};

#[derive(Debug)]
pub struct Pending<Driver> {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) dns_generation: Generation,
    pub(crate) driver: PhantomData<Driver>,
}

impl<Socket1, Socket2, const MAX_LEN: usize> Pending<Driver<Socket1, Socket2, Dns<MAX_LEN>>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub fn status(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns<MAX_LEN>>,
    ) -> Result<Option<Ipv4Addr>, Error<Socket1, Socket2, Dns<MAX_LEN>>> {
        driver
            .dns_status(
                self.link_generation,
                self.connection_generation,
                self.dns_generation,
            )
            .map_err(Into::into)
    }
}
