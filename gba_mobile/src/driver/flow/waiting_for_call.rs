use crate::{
    Timer,
    driver::{Request, Source, frames},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) enum WaitingForCall {
    PreviousRequest,
    WaitingForCall(u8),
}

impl WaitingForCall {
    pub(in crate::driver) fn new() -> Self {
        Self::PreviousRequest
    }

    pub(in crate::driver) fn request(
        self,
        timer: Timer,
        transfer_length: TransferLength,
    ) -> (Self, Option<Request>) {
        match self {
            Self::PreviousRequest => (
                Self::WaitingForCall(0),
                Some(Request::new_packet(
                    timer,
                    transfer_length,
                    Source::WaitForCall,
                )),
            ),
            Self::WaitingForCall(frame) if frame >= frames::ONE_SECOND => (
                Self::WaitingForCall(0),
                Some(Request::new_packet(
                    timer,
                    transfer_length,
                    Source::WaitForCall,
                )),
            ),
            Self::WaitingForCall(frame) => (Self::WaitingForCall(frame + 1), None),
        }
    }

    pub(in crate::driver) fn next(self) -> Option<Self> {
        match self {
            Self::PreviousRequest => Some(Self::WaitingForCall(0)),
            Self::WaitingForCall(_) => None,
        }
    }
}
