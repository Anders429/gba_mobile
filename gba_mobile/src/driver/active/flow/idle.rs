use super::{super::Phase, request, request::idle};
use crate::{Timer, mmio::serial::TransferLength};

#[derive(Debug)]
pub(in super::super) struct Idle {
    idle: request::Idle,
}

impl Idle {
    pub(super) fn new(transfer_length: TransferLength, timer: Timer) -> Self {
        Self {
            idle: request::Idle::new(transfer_length, timer),
        }
    }

    pub(super) fn vblank(&mut self) -> Result<(), idle::Timeout> {
        self.idle.vblank()
    }

    pub(super) fn timer(&mut self) {
        self.idle.timer()
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        phase: &mut Phase,
    ) -> Result<Option<Self>, idle::Error> {
        self.idle.serial(timer).map(|result| {
            result.map(|idle| Idle { idle }).or_else(|| {
                if let Phase::Linked { frame, .. } = phase {
                    // Reset the frame to 0 so that we will schedule an idle flow again.
                    *frame = 0;
                }
                None
            })
        })
    }
}
