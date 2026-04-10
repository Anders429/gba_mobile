use super::{Error, Internet};
use crate::{Driver, Generation, dns, socket};
use core::marker::PhantomData;

#[derive(Debug)]
pub struct Pending<Driver> {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) driver: PhantomData<Driver>,
}

impl<Socket1, Socket2, Dns> Pending<Driver<Socket1, Socket2, Dns>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub fn status(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns>,
    ) -> Result<Option<Internet<Driver<Socket1, Socket2, Dns>>>, Error<Socket1, Socket2, Dns>> {
        driver
            .as_active(self.link_generation)?
            .connection_status(self.connection_generation)
            .map(|finished| {
                finished.then(|| Internet {
                    link_generation: self.link_generation,
                    connection_generation: self.connection_generation,
                    driver: PhantomData,
                })
            })
            .map_err(Into::into)
    }
}
