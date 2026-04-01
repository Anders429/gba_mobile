use super::{Error, Internet};
use crate::{Driver, Generation, socket};
use core::marker::PhantomData;

#[derive(Debug)]
pub struct Pending<Driver> {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) driver: PhantomData<Driver>,
}

impl<Socket1, Socket2> Pending<Driver<Socket1, Socket2>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub fn status(
        &self,
        driver: &mut Driver<Socket1, Socket2>,
    ) -> Result<Option<Internet<Driver<Socket1, Socket2>>>, Error> {
        driver
            .connection_status(self.link_generation, self.connection_generation)
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
