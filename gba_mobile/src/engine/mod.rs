mod adapter;
mod command;
mod error;
mod flow;
mod request;
mod sink;
mod source;

use either::Either;
pub(crate) use error::Error;

use crate::{
    Timer,
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
    Error(Error),
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
    pub unsafe fn link_p2p(&mut self) {
        // TODO: Close any previous sessions.
        self.enable_communication();

        self.state = State::LinkingP2P {
            adapter: Adapter::Blue,
            transfer_length: TransferLength::_8Bit,

            request: None,
            flow: flow::LinkingP2P::Waking,
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
            State::Error(_) => {}
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
            State::Error(_) => {}
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
                            self.state = State::Error(Error::Request(request_error))
                        }
                        Err(Either::Right(command_error)) => todo!(),
                    }
                }
            }
            State::P2P => todo!(),
            State::Error(_) => {}
        }
    }
}
