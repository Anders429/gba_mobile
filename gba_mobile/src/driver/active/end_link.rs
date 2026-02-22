use crate::{
    Timer,
    driver::{Request, Source},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
pub(super) enum State {
    EndSession,
    WaitForIdle,
}

impl State {
    pub(super) fn new() -> Self {
        Self::EndSession
    }

    pub(super) fn request(self, timer: Timer, transfer_length: TransferLength) -> Request {
        match self {
            Self::EndSession => Request::new_packet(timer, transfer_length, Source::EndSession),
            Self::WaitForIdle => Request::new_wait_for_idle(),
        }
    }

    pub(super) fn next(self) -> Option<Self> {
        match self {
            Self::EndSession => Some(Self::WaitForIdle),
            Self::WaitForIdle => None,
        }
    }
}
