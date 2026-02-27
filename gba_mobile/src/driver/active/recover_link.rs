use crate::{
    Timer, driver,
    driver::{Command, HANDSHAKE, Request, sink},
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

    pub(super) fn request(self, timer: Timer, transfer_length: TransferLength) -> Request<Source> {
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

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) enum Source {
    BeginSession,
    EnableSio32,
}

impl driver::Source for Source {
    type Context = ();

    fn command(self) -> Command {
        match self {
            Self::BeginSession => Command::BeginSession,
            Self::EnableSio32 => Command::Sio32Mode,
        }
    }

    fn length(self, _context: &Self::Context) -> u8 {
        match self {
            Self::BeginSession => HANDSHAKE.len() as u8,
            Self::EnableSio32 => 1,
        }
    }

    fn get(self, index: u8, _context: &Self::Context) -> u8 {
        match self {
            Self::BeginSession => HANDSHAKE.get(index as usize).copied().unwrap_or(0x00),
            Self::EnableSio32 => 0x01,
        }
    }

    fn sink(self) -> sink::Command {
        match self {
            Self::BeginSession => sink::Command::BeginSession,
            Self::EnableSio32 => sink::Command::EnableSio32,
        }
    }
}
