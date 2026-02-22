pub(crate) mod error;

mod active;
mod adapter;
mod command;
mod frames;
mod request;
mod sink;
mod source;

use crate::{
    ArrayVec, Generation, Timer, link,
    mmio::{
        interrupt,
        serial::{self, RCNT, SIOCNT, TransferLength},
    },
    phone_number::Digit,
};
use active::Active;
use adapter::Adapter;
use command::Command;
use core::mem;
use either::Either;
use request::Request;
use source::Source;

/// Handshake for beginning a session.
const HANDSHAKE: [u8; 8] = [0x4e, 0x49, 0x4e, 0x54, 0x45, 0x4e, 0x44, 0x4f];

#[derive(Debug)]
enum State {
    NotConnected,
    Active(Active),

    CommandError(command::Error),
    RequestTimeout(request::Timeout),
    RequestError(request::Error),
}

impl State {
    fn take(&mut self) -> Self {
        mem::replace(self, Self::NotConnected)
    }
}

#[derive(Debug)]
pub struct Driver {
    state: State,
    timer: Timer,
    generation: Generation,
}

impl Driver {
    /// Create a new communication driver.
    pub const fn new(timer: Timer) -> Self {
        Self {
            state: State::NotConnected,
            timer,
            generation: Generation::new(),
        }
    }

    /// Enables communication, if it isn't already enabled.
    fn enable_communication(&self) {
        unsafe {
            // Set transfer mode to 8-bit Normal.
            RCNT.write_volatile(serial::Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(TransferLength::_8Bit));

            // Enable interrupts for vblank, timer, and serial.
            let timer_enable = match self.timer {
                Timer::_0 => interrupt::Enable::TIMER0,
                Timer::_1 => interrupt::Enable::TIMER1,
                Timer::_2 => interrupt::Enable::TIMER2,
                Timer::_3 => interrupt::Enable::TIMER3,
            };
            interrupt::ENABLE.write_volatile(
                interrupt::ENABLE.read_volatile()
                    | interrupt::Enable::VBLANK
                    | timer_enable
                    | interrupt::Enable::SERIAL,
            );
        }
    }

    /// # Safety
    /// Must take exclusive ownership over the serial registers and the timer registers related to
    /// the [`Timer`] used to construct this Engine.
    pub unsafe fn link(&mut self) -> link::Pending {
        let old_state = self.state.take();
        self.state = match old_state {
            State::NotConnected
            | State::CommandError(_)
            | State::RequestTimeout(_)
            | State::RequestError(_) => {
                self.enable_communication();
                State::Active(Active::new(self.timer))
            }
            State::Active(active) => State::Active(active.reset_link(self.timer)),
        };

        self.generation = self.generation.increment();

        link::Pending {
            generation: self.generation,
        }
    }

    pub(crate) fn linking_status(
        &self,
        generation: Generation,
    ) -> Result<bool, error::link::Error> {
        if generation != self.generation {
            return Err(error::link::Error::superseded());
        }

        match &self.state {
            State::NotConnected => Err(error::link::Error::closed()),
            State::Active(active) => active.linking_status(),
            State::CommandError(error) => Err(error.clone().into()),
            State::RequestTimeout(timeout) => Err(timeout.clone().into()),
            State::RequestError(error) => Err(error.clone().into()),
        }
    }

    pub(crate) fn wait_for_call(
        &mut self,
        generation: Generation,
    ) -> Result<Generation, error::link::Error> {
        if generation != self.generation {
            return Err(error::link::Error::superseded());
        }

        let state = self.state.take();
        match state {
            State::Active(active) => {
                let (new_active, call_generation) = active.wait_for_call()?;
                self.state = State::Active(new_active);
                Ok(call_generation)
            }
            State::NotConnected => Err(error::link::Error::closed()),
            State::CommandError(error) => Err(error.clone().into()),
            State::RequestTimeout(timeout) => Err(timeout.clone().into()),
            State::RequestError(error) => Err(error.clone().into()),
        }
    }

    pub(crate) fn call(
        &mut self,
        phone_number: ArrayVec<Digit, 32>,
        generation: Generation,
    ) -> Result<Generation, error::link::Error> {
        if generation != self.generation {
            return Err(error::link::Error::superseded());
        }

        let state = self.state.take();
        match state {
            State::Active(active) => {
                let (new_active, call_generation) = active.call(phone_number, self.timer)?;
                self.state = State::Active(new_active);
                Ok(call_generation)
            }
            State::NotConnected => Err(error::link::Error::closed()),
            State::CommandError(error) => Err(error.clone().into()),
            State::RequestTimeout(timeout) => Err(timeout.clone().into()),
            State::RequestError(error) => Err(error.clone().into()),
        }
    }

    pub(crate) fn p2p_status(
        &self,
        generation: Generation,
        call_generation: Generation,
    ) -> Result<bool, error::p2p::Error> {
        if generation != self.generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::NotConnected => Err(error::link::Error::closed().into()),
            State::Active(active) => active.p2p_status(call_generation),
            State::CommandError(error) => Err(error::link::Error::from(error.clone()).into()),
            State::RequestTimeout(timeout) => Err(error::link::Error::from(timeout.clone()).into()),
            State::RequestError(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn end_session(&mut self, generation: Generation) {
        if generation != self.generation {
            // This request came from an old connection. We should not honor it, since that old
            // connection is already disconnected.
            return;
        }

        let old_state = self.state.take();
        self.state = match old_state {
            State::NotConnected
            | State::CommandError(_)
            | State::RequestTimeout(_)
            | State::RequestError(_) => State::NotConnected,
            State::Active(active) => State::Active(active.end_link(self.timer)),
        }
    }

    pub fn vblank(&mut self) {
        match &mut self.state {
            State::NotConnected => {}
            State::Active(active) => {
                if let Err(timeout) = active.vblank(self.timer) {
                    self.state = State::RequestTimeout(timeout);
                }
            }
            State::CommandError(_) => {}
            State::RequestTimeout(_) => {}
            State::RequestError(_) => {}
        }
    }

    pub fn timer(&mut self) {
        self.timer.stop();
        match &mut self.state {
            State::NotConnected => {}
            State::Active(active) => active.timer(),
            State::CommandError(_) => {}
            State::RequestTimeout(_) => {}
            State::RequestError(_) => {}
        }
    }

    pub fn serial(&mut self) {
        let state = self.state.take();

        self.state = match state {
            State::NotConnected => State::NotConnected,
            State::Active(active) => match active.serial(self.timer) {
                Ok(Some(active)) => State::Active(active),
                Ok(None) => State::NotConnected,
                Err(Either::Left(request_error)) => State::RequestError(request_error),
                Err(Either::Right(command_error)) => State::CommandError(command_error),
            },
            State::CommandError(error) => State::CommandError(error),
            State::RequestTimeout(timeout) => State::RequestTimeout(timeout),
            State::RequestError(error) => State::RequestError(error),
        }
    }
}
