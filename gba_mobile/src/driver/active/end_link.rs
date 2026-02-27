use crate::{
    Timer, driver,
    driver::{Command, Request, sink},
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

    pub(super) fn request(self, timer: Timer, transfer_length: TransferLength) -> Request<Source> {
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

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) enum Source {
    EndSession,
}

impl driver::Source for Source {
    type Context = ();

    fn command(self) -> Command {
        match self {
            Self::EndSession => Command::EndSession,
        }
    }

    fn length(self, _context: &Self::Context) -> u8 {
        match self {
            Self::EndSession => 0,
        }
    }

    fn get(self, _index: u8, _context: &Self::Context) -> u8 {
        match self {
            Self::EndSession => 0x00,
        }
    }

    fn sink(self) -> sink::Command {
        match self {
            Self::EndSession => sink::Command::EndSession,
        }
    }
}
