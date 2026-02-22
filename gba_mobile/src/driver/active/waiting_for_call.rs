use crate::{
    Timer,
    driver::{Request, Source, frames},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct State(u8);

impl State {
    pub(super) fn new() -> Self {
        Self(0)
    }

    pub(super) fn request(
        self,
        timer: Timer,
        transfer_length: TransferLength,
    ) -> (Self, Option<Request>) {
        if self.0 >= frames::ONE_SECOND {
            (
                Self(0),
                Some(Request::new_packet(
                    timer,
                    transfer_length,
                    Source::WaitForCall,
                )),
            )
        } else {
            (Self(self.0 + 1), None)
        }
    }
}
