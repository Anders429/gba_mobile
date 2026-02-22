use crate::{
    Timer,
    driver::{Request, Source},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
pub(super) enum State {
    ResetSession,
    ResetWaitForIdle,
    Sio32,
    Sio32WaitForIdle,
}

impl State {
    pub(super) fn new() -> Self {
        Self::ResetSession
    }

    pub(super) fn request(self, timer: Timer, transfer_length: TransferLength) -> Request {
        match self {
            Self::ResetSession => Request::new_packet(timer, transfer_length, Source::Reset),
            Self::ResetWaitForIdle => Request::new_wait_for_idle(),
            Self::Sio32 => Request::new_packet(timer, transfer_length, Source::EnableSio32),
            Self::Sio32WaitForIdle => Request::new_wait_for_idle(),
        }
    }

    pub(super) fn next(self) -> Option<Self> {
        match self {
            Self::ResetSession => Some(Self::ResetWaitForIdle),
            Self::ResetWaitForIdle => Some(Self::Sio32),
            Self::Sio32 => Some(Self::Sio32WaitForIdle),
            Self::Sio32WaitForIdle => None,
        }
    }
}
