use super::{Idle, idle};
use crate::{Timer, mmio::serial::TransferLength};

#[derive(Debug)]
pub(in crate::driver::active) enum RepeatingIdle {
    Idle(Idle),
    Delay {
        transfer_length: TransferLength,
        timer: Timer,
    },
}

impl RepeatingIdle {
    pub(in crate::driver::active::flow) fn new(
        transfer_length: TransferLength,
        timer: Timer,
    ) -> Self {
        Self::Idle(Idle::new(transfer_length, timer))
    }

    pub(in crate::driver::active::flow) fn vblank(&mut self) -> Result<(), idle::Timeout> {
        match self {
            Self::Idle(idle) => idle.vblank(),
            Self::Delay { .. } => {
                // Send a new idle every frame.
                Ok(())
            }
        }
    }

    pub(in crate::driver::active::flow) fn timer(&mut self) {
        match self {
            Self::Idle(idle) => idle.timer(),
            Self::Delay { .. } => {}
        }
    }

    pub(in crate::driver::active::flow) fn serial(self, timer: Timer) -> Result<Self, idle::Error> {
        match self {
            Self::Idle(idle) => {
                let transfer_length = idle.transfer_length;
                Ok(idle
                    .serial(timer)?
                    .map(|idle| Self::Idle(idle))
                    .unwrap_or_else(|| Self::Delay {
                        transfer_length,
                        timer,
                    }))
            }
            Self::Delay {
                transfer_length,
                timer,
            } => Ok(Self::Delay {
                transfer_length,
                timer,
            }),
        }
    }
}
