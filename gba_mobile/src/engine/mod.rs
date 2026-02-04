mod adapter;
mod command;
mod error;
mod flow;
mod request;
mod sink;
mod source;

pub(crate) use error::Error;

use crate::mmio::serial::TransferLength;
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
}

impl Engine {
    /// Create a new communication engine.
    pub const unsafe fn new() -> Self {
        Self {
            state: State::NotConnected,
        }
    }

    pub fn link_p2p(&mut self) {
        // TODO: Close any previous sessions.
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
                    request.vblank();
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
            State::LinkingP2P { request, .. } => {
                request.as_mut().map(|request| request.timer());
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
                ..
            } => {
                if let Some(request) = state_request.take() {
                    match request.serial(adapter) {
                        Ok(next_request) => *state_request = next_request,
                        Err(error) => self.state = State::Error(Error::Request(error)),
                    }
                }
            }
            State::P2P => todo!(),
            State::Error(_) => {}
        }
    }
}
