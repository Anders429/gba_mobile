use crate::{
    Timer,
    driver::{Request, Source},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
pub(super) enum State {
    EndSessionWaitForIdle,
    BeginSession,
    Sio32,
    Sio32WaitForIdle,
}

impl State {
    pub(super) fn new() -> Self {
        Self::EndSessionWaitForIdle
    }

    pub(super) fn request(self, timer: Timer, transfer_length: TransferLength) -> Request {
        match self {
            Self::EndSessionWaitForIdle => Request::new_wait_for_idle(),
            Self::BeginSession => Request::new_packet(timer, transfer_length, Source::BeginSession),
            Self::Sio32 => Request::new_packet(timer, transfer_length, Source::EnableSio32),
            Self::Sio32WaitForIdle => Request::new_wait_for_idle(),
        }
    }

    pub(super) fn next(self) -> Option<Self> {
        match self {
            Self::EndSessionWaitForIdle => Some(Self::BeginSession),
            Self::BeginSession => Some(Self::Sio32),
            Self::Sio32 => Some(Self::Sio32WaitForIdle),
            Self::Sio32WaitForIdle => None,
        }
    }
}
