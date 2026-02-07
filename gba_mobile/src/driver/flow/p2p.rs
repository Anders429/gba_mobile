use crate::{Timer, driver::Request, mmio::serial::TransferLength};

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) struct P2P(u8);

impl P2P {
    pub(in crate::driver) const NONE: Self = Self(0b0000_0000);
    pub(in crate::driver) const IDLE: Self = Self(0b0000_0001);
}

impl P2P {
    pub(in crate::driver) fn request(
        self,
        timer: Timer,
        transfer_length: TransferLength,
    ) -> (Self, Option<Request>) {
        if self.0 == Self::IDLE.0 {
            (Self::NONE, Some(Request::new_idle(timer, transfer_length)))
        } else {
            (self, None)
        }
    }
}
