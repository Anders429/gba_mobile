use core::marker::PhantomData;

use crate::{
    Driver, Generation, dns,
    link::{Error, Link},
    socket,
};

#[derive(Debug)]
pub struct Pending<Driver> {
    pub(crate) link_generation: Generation,
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
        driver: &Driver<Socket1, Socket2, Dns>,
    ) -> Result<Option<Link<Driver<Socket1, Socket2, Dns>>>, Error> {
        driver
            .link_status(self.link_generation)
            .map(|finished| {
                finished.then(|| Link {
                    link_generation: self.link_generation,
                    driver: PhantomData,
                })
            })
            .map_err(|error| error.into())
    }

    /// Cancel this pending link.
    pub fn cancel(self) {
        todo!()
    }
}
