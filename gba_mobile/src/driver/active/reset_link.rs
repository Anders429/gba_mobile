use crate::{
    Timer, driver,
    driver::{Command, Request, sink},
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

    pub(super) fn request(self, timer: Timer, transfer_length: TransferLength) -> Request<Source> {
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

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) enum Source {
    Reset,
    EnableSio32,
}

impl driver::Source for Source {
    type Context = ();

    fn command(self) -> Command {
        match self {
            Self::Reset => Command::Reset,
            Self::EnableSio32 => Command::Sio32Mode,
        }
    }

    fn length(self, _context: &Self::Context) -> u8 {
        match self {
            Self::Reset => 0,
            Self::EnableSio32 => 1,
        }
    }

    fn get(self, _index: u8, _context: &Self::Context) -> u8 {
        match self {
            Self::Reset => 0x00,
            Self::EnableSio32 => 0x01,
        }
    }

    fn sink(self) -> sink::Command {
        match self {
            Self::Reset => sink::Command::Reset,
            Self::EnableSio32 => sink::Command::EnableSio32,
        }
    }
}
