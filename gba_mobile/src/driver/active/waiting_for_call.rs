use crate::{
    Timer, driver,
    driver::{Command, Request, frames, sink},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct State(u8);

impl State {
    pub(super) fn new() -> Self {
        Self(0)
    }

    pub(super) fn request(
        self,
        timer: Timer,
        transfer_length: TransferLength,
    ) -> (Self, Option<Request<Source>>) {
        if self.0 >= frames::ONE_SECOND {
            (
                Self(0),
                Some(Request::new_packet(
                    timer,
                    transfer_length,
                    Source::WaitForCall,
                )),
            )
        } else {
            (Self(self.0 + 1), None)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) enum Source {
    WaitForCall,
}

impl driver::Source for Source {
    type Context = ();

    fn command(self) -> Command {
        match self {
            Self::WaitForCall => Command::WaitForTelephoneCall,
        }
    }

    fn length(self, _context: &Self::Context) -> u8 {
        match self {
            Self::WaitForCall => 0,
        }
    }

    fn get(self, _index: u8, _context: &Self::Context) -> u8 {
        match self {
            Self::WaitForCall => 0x00,
        }
    }

    fn sink(self) -> sink::Command {
        match self {
            Self::WaitForCall => sink::Command::WaitForCall,
        }
    }
}
