pub mod error;
pub mod mode;

mod pending;

pub use mode::{Mode, P2p, Socket1, Socket2};

use crate::{Driver, Generation, config, dns, socket};
use core::marker::PhantomData;

#[derive(Debug)]
pub struct Connection<Driver, Mode> {
    pub(crate) link_generation: Generation,
    pub(crate) connection_generation: Generation,
    pub(crate) mode: Mode,
    pub(crate) driver: PhantomData<Driver>,
}

impl<Socket1, Socket2, Dns, Config, Mode> Connection<Driver<Socket1, Socket2, Dns, Config>, Mode>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
    Mode: self::Mode<Socket1, Socket2, Dns, Config>,
{
    pub fn read(
        &mut self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
        buf: &mut [u8],
    ) -> Result<usize, Mode::IoError> {
        self.mode.read(
            driver,
            self.link_generation,
            self.connection_generation,
            buf,
        )
    }

    pub fn write(
        &mut self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
        buf: &[u8],
    ) -> Result<usize, Mode::Error> {
        self.mode.write(
            driver,
            self.link_generation,
            self.connection_generation,
            buf,
        )
    }

    pub fn flush(
        &mut self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Mode::Error> {
        self.mode
            .flush(driver, self.link_generation, self.connection_generation)
    }

    pub fn close(
        &self,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Mode::Error> {
        self.mode
            .close(driver, self.link_generation, self.connection_generation)
    }
}
