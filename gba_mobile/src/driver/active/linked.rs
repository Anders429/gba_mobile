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
            (Self(0), Some(Request::new_idle(timer, transfer_length)))
        } else {
            (Self(self.0 + 1), None)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) enum Source {}

impl driver::Source for Source {
    type Context = ();

    fn command(self) -> Command {
        unimplemented!()
    }

    fn length(self, _context: &Self::Context) -> u8 {
        unimplemented!()
    }

    fn get(self, _index: u8, _context: &Self::Context) -> u8 {
        unimplemented!()
    }

    fn sink(self) -> sink::Command {
        unimplemented!()
    }
}
