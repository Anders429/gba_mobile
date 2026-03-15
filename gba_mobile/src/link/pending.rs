use crate::{
    DRIVER, Generation,
    link::{Error, Link},
    mmio::interrupt,
};

#[derive(Debug)]
pub struct Pending {
    pub(crate) link_generation: Generation,
}

impl Pending {
    pub fn status(&self) -> Result<Option<Link>, Error> {
        unsafe {
            let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
            interrupt::MASTER_ENABLE.write_volatile(false);
            let result = DRIVER
                .link_status(self.link_generation)
                .map(|finished| {
                    finished.then(|| Link {
                        link_generation: self.link_generation,
                    })
                })
                .map_err(|error| error.into());
            interrupt::MASTER_ENABLE.write_volatile(prev_enable);
            result
        }
    }

    /// Cancel this pending link.
    pub fn cancel(self) {
        todo!()
    }
}
