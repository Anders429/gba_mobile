use crate::{
    Timer,
    driver::{Request, frames},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) struct Linked(u8);

impl Linked {
    pub(in crate::driver) fn new() -> Self {
        Self(0)
    }

    pub(in crate::driver) fn request(
        self,
        timer: Timer,
        transfer_length: TransferLength,
    ) -> (Self, Option<Request>) {
        if self.0 >= frames::ONE_SECOND {
            (Self(0), Some(Request::new_idle(timer, transfer_length)))
        } else {
            (Self(self.0 + 1), None)
        }
    }
}
