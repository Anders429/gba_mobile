pub(crate) mod error;

mod adapter;
mod command;
mod flow;
mod request;
mod sink;
mod source;

use either::Either;

use crate::{
    Generation, Timer, link_p2p,
    mmio::{
        interrupt,
        serial::{self, RCNT, SIOCNT, TransferLength},
    },
};
use adapter::Adapter;
use command::Command;
use request::Request;
use source::Source;

/// Handshake for beginning a session.
const HANDSHAKE: [u8; 8] = [0x4e, 0x49, 0x4e, 0x54, 0x45, 0x4e, 0x44, 0x4f];

#[derive(Debug)]
enum State {
    NotConnected,
    LinkingP2P {
        adapter: Adapter,
        transfer_length: TransferLength,

        request: Option<Request>,
        flow: flow::LinkingP2P,
    },
    P2P,
    LinkingP2PError(command::Error),
    RequestTimeout(request::Timeout),
    RequestError(request::Error),
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
                interrupt::Enable::VBLANK | timer_enable | interrupt::Enable::SERIAL,
            );
        }
    }

    /// # Safety
    /// Must take exclusive ownership over the serial registers and the timer registers related to
    /// the [`Timer`] used to construct this Engine.
    pub unsafe fn link_p2p(&mut self) -> link_p2p::Pending {
        // TODO: Close any previous sessions.
        self.enable_communication();

        self.state = State::LinkingP2P {
            adapter: Adapter::Blue,
            transfer_length: TransferLength::_8Bit,

            request: None,
            flow: flow::LinkingP2P::Waking,
        };
        self.generation = self.generation.increment();

        link_p2p::Pending {
            generation: self.generation,
        }
    }

    pub(crate) fn link_p2p_status(
        &self,
        generation: Generation,
    ) -> Result<bool, error::link_p2p::Error> {
        if generation != self.generation {
            return Err(error::link_p2p::Error::superseded());
        }

        match &self.state {
            State::NotConnected => Err(error::link_p2p::Error::aborted()),
            State::LinkingP2P { .. } => Ok(false),
            State::P2P => Ok(true),
            State::LinkingP2PError(error) => Err(error.clone().into()),
            State::RequestTimeout(timeout) => Err(timeout.clone().into()),
            State::RequestError(error) => Err(error.clone().into()),
        }
    }

    pub fn vblank(&mut self) {
        match &mut self.state {
            State::NotConnected => {}
            State::LinkingP2P {
                transfer_length,
                request,
                flow,
                ..
            } => {
                if let Some(request) = request {
                    if let Err(timeout) = request.vblank(*transfer_length) {
                        self.state = State::RequestTimeout(timeout);
                    }
                } else {
                    // Schedule a new request.
                    log::info!("Scheduling new request");
                    *request = Some(flow.request(self.timer, *transfer_length));
                }
            }
            State::P2P => todo!(),
            State::LinkingP2PError(_) => {}
            State::RequestTimeout(_) => {}
            State::RequestError(_) => {}
        }
    }

    pub fn timer(&mut self) {
        self.timer.stop();
        match &mut self.state {
            State::NotConnected => {}
            State::LinkingP2P {
                request,
                transfer_length,
                ..
            } => {
                request
                    .as_mut()
                    .map(|request| request.timer(*transfer_length));
            }
            State::P2P => todo!(),
            State::LinkingP2PError(_) => {}
            State::RequestTimeout(_) => {}
            State::RequestError(_) => {}
        }
    }

    pub fn serial(&mut self) {
        match &mut self.state {
            State::NotConnected => {}
            State::LinkingP2P {
                request: state_request,
                adapter,
                transfer_length,
                flow,
                ..
            } => {
                if let Some(request) = state_request.take() {
                    match request.serial(adapter, transfer_length, self.timer) {
                        Ok(Some(next_request)) => *state_request = Some(next_request),
                        Ok(None) => match flow.next() {
                            Some(next_flow) => *flow = next_flow,
                            None => self.state = State::P2P,
                        },
                        Err(Either::Left(request_error)) => {
                            self.state = State::RequestError(request_error)
                        }
                        Err(Either::Right(command_error)) => {
                            self.state = State::LinkingP2PError(command_error)
                        }
                    }
                }
            }
            State::P2P => todo!(),
            State::LinkingP2PError(_) => {}
            State::RequestTimeout(_) => {}
            State::RequestError(_) => {}
        }
    }
}
