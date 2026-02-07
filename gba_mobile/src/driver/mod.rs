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
use core::mem;
use request::Request;
use source::Source;

/// Handshake for beginning a session.
const HANDSHAKE: [u8; 8] = [0x4e, 0x49, 0x4e, 0x54, 0x45, 0x4e, 0x44, 0x4f];
const FRAMES_1_SECOND: u8 = 60;

#[derive(Debug)]
enum State {
    NotConnected,
    LinkingP2P {
        adapter: Adapter,
        transfer_length: TransferLength,

        request: Option<Request>,
        flow: flow::LinkingP2P,
    },
    P2P {
        adapter: Adapter,
        transfer_length: TransferLength,

        request: Option<Request>,
        flow: flow::P2P,

        frame: u8,
    },

    EndSession {
        adapter: Adapter,
        transfer_length: TransferLength,

        request: Option<Request>,
        flow: flow::EndSession,
    },

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
    pub unsafe fn link_p2p(&mut self) -> link_p2p::Pending {
        let old_state = self.state.take();
        self.state = match old_state {
            State::NotConnected
            | State::CommandError(_)
            | State::RequestTimeout(_)
            | State::RequestError(_) => {
                self.enable_communication();
                State::LinkingP2P {
                    adapter: Adapter::Blue,
                    transfer_length: TransferLength::_8Bit,

                    request: None,
                    flow: flow::LinkingP2P::Waking,
                }
            }
            State::LinkingP2P {
                adapter,
                transfer_length,
                request,
                ..
            }
            | State::P2P {
                adapter,
                transfer_length,
                request,
                ..
            } => State::EndSession {
                adapter,
                transfer_length,

                request,
                flow: flow::EndSession::new(flow::end_session::Destination::LinkingP2P),
            },
            State::EndSession {
                adapter,
                transfer_length,
                request,
                flow,
            } => State::EndSession {
                adapter,
                transfer_length,

                request,
                flow: flow.set_destination(flow::end_session::Destination::LinkingP2P),
            },
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
            State::P2P { .. } => Ok(true),
            State::EndSession { .. } => Ok(false),
            State::CommandError(error) => Err(error.clone().into()),
            State::RequestTimeout(timeout) => Err(timeout.clone().into()),
            State::RequestError(error) => Err(error.clone().into()),
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
            State::LinkingP2P {
                adapter,
                transfer_length,
                request,
                ..
            }
            | State::P2P {
                adapter,
                transfer_length,
                request,
                ..
            } => State::EndSession {
                adapter,
                transfer_length,

                request,
                flow: flow::EndSession::new(flow::end_session::Destination::NotConnected),
            },
            State::EndSession {
                adapter,
                transfer_length,
                request,
                flow,
            } => State::EndSession {
                adapter,
                transfer_length,

                request,
                flow: flow.set_destination(flow::end_session::Destination::NotConnected),
            },
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
                    *request = Some(flow.request(self.timer, *transfer_length));
                }
            }
            State::P2P {
                frame,
                flow,
                transfer_length,
                request,
                ..
            } => {
                if *frame >= FRAMES_1_SECOND {
                    *flow = flow::P2P::IDLE;
                    *frame = 0;
                } else {
                    *frame += 1;
                }

                if let Some(request) = request {
                    if let Err(timeout) = request.vblank(*transfer_length) {
                        self.state = State::RequestTimeout(timeout);
                    }
                } else {
                    // Schedule a new request.
                    let (new_flow, new_request) = flow.request(self.timer, *transfer_length);
                    *flow = new_flow;
                    *request = new_request;
                }
            }
            State::EndSession {
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
                    *request = Some(flow.request(self.timer, *transfer_length));
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
            State::LinkingP2P {
                request,
                transfer_length,
                ..
            } => {
                request
                    .as_mut()
                    .map(|request| request.timer(*transfer_length));
            }
            State::P2P {
                request,
                transfer_length,
                ..
            } => {
                request
                    .as_mut()
                    .map(|request| request.timer(*transfer_length));
            }
            State::EndSession {
                request,
                transfer_length,
                ..
            } => {
                request
                    .as_mut()
                    .map(|request| request.timer(*transfer_length));
            }
            State::CommandError(_) => {}
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
                            None => {
                                self.state = State::P2P {
                                    adapter: *adapter,
                                    transfer_length: *transfer_length,

                                    request: None,
                                    flow: flow::P2P::NONE,

                                    frame: 0,
                                }
                            }
                        },
                        Err(Either::Left(request_error)) => {
                            self.state = State::RequestError(request_error)
                        }
                        Err(Either::Right(command_error)) => {
                            self.state = State::CommandError(command_error)
                        }
                    }
                }
            }
            State::P2P {
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
                            self.state = State::CommandError(command_error)
                        }
                    }
                }
            }
            State::EndSession {
                adapter,
                transfer_length,
                request: state_request,
                flow,
            } => {
                if let Some(request) = state_request.take() {
                    match request.serial(adapter, transfer_length, self.timer) {
                        Ok(Some(next_request)) => *state_request = Some(next_request),
                        Ok(None) => match flow.next() {
                            Some(next_flow) => *flow = next_flow,
                            None => {
                                self.state = match flow.destination() {
                                    flow::end_session::Destination::NotConnected => {
                                        State::NotConnected
                                    }
                                    flow::end_session::Destination::LinkingP2P => {
                                        State::LinkingP2P {
                                            adapter: *adapter,
                                            transfer_length: *transfer_length,
                                            request: None,
                                            flow: flow::LinkingP2P::BeginSession,
                                        }
                                    }
                                }
                            }
                        },
                        Err(Either::Left(request_error)) => {
                            self.state = State::RequestError(request_error)
                        }
                        Err(Either::Right(command_error)) => {
                            self.state = State::CommandError(command_error)
                        }
                    }
                }
            }
            State::CommandError(_) => {}
            State::RequestTimeout(_) => {}
            State::RequestError(_) => {}
        }
    }
}
