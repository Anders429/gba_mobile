pub(crate) mod error;

mod adapter;
mod command;
mod flow;
mod request;
mod sink;
mod source;

use either::Either;

use crate::{
    Timer,
    link_p2p::{self, LinkP2P},
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
    RequestError(request::Error),
}

#[derive(Debug)]
pub struct Engine {
    state: State,
    timer: Timer,
}

impl Engine {
    /// Create a new communication engine.
    pub const fn new(timer: Timer) -> Self {
        Self {
            state: State::NotConnected,
            timer,
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

        link_p2p::Pending {}
    }

    pub(crate) fn link_p2p_status(&self) -> Result<bool, error::link_p2p::Error> {
        match &self.state {
            State::NotConnected => Err(error::link_p2p::Error::aborted()),
            State::LinkingP2P { .. } => Ok(false),
            State::P2P => Ok(true),
            State::LinkingP2PError(error) => Err(error.clone().into()),
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
                    request.vblank(*transfer_length);
                } else {
                    // Schedule a new request.
                    *request = Some(flow.request(*transfer_length));
                }
            }
            State::P2P => todo!(),
            State::LinkingP2PError(_) => {}
            State::RequestError(_) => {}
        }
    }

    pub fn timer(&mut self) {
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
                ..
            } => {
                if let Some(request) = state_request.take() {
                    match request.serial(adapter, transfer_length, self.timer) {
                        Ok(next_request) => *state_request = next_request,
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
            State::RequestError(_) => {}
        }
    }
}
