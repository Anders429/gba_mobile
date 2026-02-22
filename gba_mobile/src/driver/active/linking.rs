use crate::{
    Timer,
    driver::{Request, Source},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
pub(super) enum State {
    Waking,
    BeginSession,
    Sio32,
    WaitForIdle,
}

impl State {
    pub(super) fn new() -> Self {
        Self::Waking
    }

    pub(super) fn request(self, timer: Timer, transfer_length: TransferLength) -> Request {
        match self {
            Self::Waking => Request::new_wait_for_idle(),
            Self::BeginSession => Request::new_packet(timer, transfer_length, Source::BeginSession),
            Self::Sio32 => Request::new_packet(timer, transfer_length, Source::EnableSio32),
            Self::WaitForIdle => Request::new_wait_for_idle(),
        }
    }

    pub(super) fn next(self) -> Option<Self> {
        match self {
            Self::Waking => Some(Self::BeginSession),
            Self::BeginSession => Some(Self::Sio32),
            Self::Sio32 => Some(Self::WaitForIdle),
            Self::WaitForIdle => None,
        }
    }
}
