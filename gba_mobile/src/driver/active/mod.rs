mod end_link;
mod linked;
mod linking;
mod recover_link;
mod reset_link;
mod waiting_for_call;

use crate::{
    Generation, Timer,
    arrayvec::ArrayVec,
    driver::{Adapter, Request, Source, command, error, request},
    mmio::serial::TransferLength,
    phone_number::Digit,
};
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct Active {
    adapter: Adapter,
    transfer_length: TransferLength,

    state: State,
}

impl Active {
    pub(in crate::driver) fn new(timer: Timer) -> Self {
        let transfer_length = TransferLength::_8Bit;
        let linking_state = linking::State::new();
        Self {
            adapter: Adapter::Blue,
            transfer_length,

            state: State::Linking {
                request: linking_state.request(timer, transfer_length),
                state: linking_state,
            },
        }
    }

    pub(in crate::driver) fn linking_status(&self) -> Result<bool, error::link::Error> {
        self.state.linking_status()
    }

    pub(in crate::driver) fn p2p_status(
        &self,
        call_generation: Generation,
    ) -> Result<bool, error::p2p::Error> {
        self.state.p2p_status(call_generation)
    }

    pub(in crate::driver) fn reset_link(mut self, timer: Timer) -> Self {
        self.state = self.state.reset_link(timer, self.transfer_length);
        self
    }

    pub(in crate::driver) fn end_link(mut self, timer: Timer) -> Self {
        self.state = self.state.end_link(timer, self.transfer_length);
        self
    }

    pub(in crate::driver) fn wait_for_call(
        mut self,
    ) -> Result<(Self, Generation), error::link::Error> {
        let (state, call_generation) = self.state.wait_for_call()?;
        self.state = state;

        Ok((self, call_generation))
    }

    pub(in crate::driver) fn call(
        mut self,
        phone_number: ArrayVec<Digit, 32>,
        timer: Timer,
    ) -> Result<(Self, Generation), error::link::Error> {
        let (state, call_generation) =
            self.state
                .call(phone_number, timer, self.transfer_length, self.adapter)?;
        self.state = state;

        Ok((self, call_generation))
    }

    pub(in crate::driver) fn vblank(&mut self, timer: Timer) -> Result<(), request::Timeout> {
        self.state.vblank(self.transfer_length, timer)
    }

    pub(in crate::driver) fn timer(&mut self) {
        self.state.timer(self.transfer_length)
    }

    pub(in crate::driver) fn serial(
        mut self,
        timer: Timer,
    ) -> Result<Option<Self>, Either<request::Error, command::Error>> {
        self.state = if let Some(state) =
            self.state
                .serial(&mut self.adapter, &mut self.transfer_length, timer)?
        {
            state
        } else {
            return Ok(None);
        };
        Ok(Some(self))
    }
}

/// A request from a previous state.
///
/// We still need to process this, but we don't care about its result.
#[derive(Debug)]
enum PreviousRequest {
    Linking(Request),
    Linked(Request),
    WaitingForCall(Request),
    Call(Request),
    ResetLink(Request),
    EndLink(Request),
    RecoverLink(Request),
}

impl PreviousRequest {
    fn vblank(&mut self, transfer_length: TransferLength) -> Result<(), request::Timeout> {
        match self {
            Self::Linking(request) => request.vblank(transfer_length),
            Self::Linked(request) => request.vblank(transfer_length),
            Self::WaitingForCall(request) => request.vblank(transfer_length),
            Self::Call(request) => request.vblank(transfer_length),
            Self::ResetLink(request) => request.vblank(transfer_length),
            Self::EndLink(request) => request.vblank(transfer_length),
            Self::RecoverLink(request) => request.vblank(transfer_length),
        }
    }

    fn timer(&mut self, transfer_length: TransferLength) {
        match self {
            Self::Linking(request) => request.timer(transfer_length),
            Self::Linked(request) => request.timer(transfer_length),
            Self::WaitingForCall(request) => request.timer(transfer_length),
            Self::Call(request) => request.timer(transfer_length),
            Self::ResetLink(request) => request.timer(transfer_length),
            Self::EndLink(request) => request.timer(transfer_length),
            Self::RecoverLink(request) => request.timer(transfer_length),
        }
    }

    fn serial(
        self,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
        timer: Timer,
    ) -> Result<Option<Self>, request::Error> {
        match self {
            Self::Linking(request) => match request.serial(adapter, transfer_length, timer) {
                Ok(new_request) => Ok(new_request.map(Self::Linking)),
                Err(Either::Left(request_error)) => Err(request_error),
                Err(Either::Right(_)) => Ok(None),
            },
            Self::Linked(request) => match request.serial(adapter, transfer_length, timer) {
                Ok(new_request) => Ok(new_request.map(Self::Linked)),
                Err(Either::Left(request_error)) => Err(request_error),
                Err(Either::Right(_)) => Ok(None),
            },
            Self::WaitingForCall(request) => {
                match request.serial(adapter, transfer_length, timer) {
                    Ok(new_request) => Ok(new_request.map(Self::WaitingForCall)),
                    Err(Either::Left(request_error)) => Err(request_error),
                    Err(Either::Right(_)) => Ok(None),
                }
            }
            Self::Call(request) => match request.serial(adapter, transfer_length, timer) {
                Ok(new_request) => Ok(new_request.map(Self::Call)),
                Err(Either::Left(request_error)) => Err(request_error),
                Err(Either::Right(_)) => Ok(None),
            },
            Self::ResetLink(request) => match request.serial(adapter, transfer_length, timer) {
                Ok(new_request) => Ok(new_request.map(Self::EndLink)),
                Err(Either::Left(request_error)) => Err(request_error),
                Err(Either::Right(_)) => Ok(None),
            },
            Self::EndLink(request) => match request.serial(adapter, transfer_length, timer) {
                Ok(new_request) => Ok(new_request.map(Self::EndLink)),
                Err(Either::Left(request_error)) => Err(request_error),
                Err(Either::Right(_)) => Ok(None),
            },
            Self::RecoverLink(request) => match request.serial(adapter, transfer_length, timer) {
                Ok(new_request) => Ok(new_request.map(Self::RecoverLink)),
                Err(Either::Left(request_error)) => Err(request_error),
                Err(Either::Right(_)) => Ok(None),
            },
        }
    }
}

#[derive(Debug)]
enum ProcessingRequest {
    Current(Request),
    Previous(PreviousRequest),
}

impl ProcessingRequest {
    fn vblank(&mut self, transfer_length: TransferLength) -> Result<(), request::Timeout> {
        match self {
            Self::Current(request) => request.vblank(transfer_length),
            Self::Previous(request) => request.vblank(transfer_length),
        }
    }

    fn timer(&mut self, transfer_length: TransferLength) {
        match self {
            Self::Current(request) => request.timer(transfer_length),
            Self::Previous(request) => request.timer(transfer_length),
        }
    }
}

#[derive(Debug)]
enum State {
    Linking {
        request: Request,
        state: linking::State,
    },
    Linked {
        request: Option<Request>,
        state: linked::State,
        call_generation: Generation,
        call_error: Option<command::Error>,
    },
    WaitingForCall {
        request: Option<ProcessingRequest>,
        state: waiting_for_call::State,
        call_generation: Generation,
    },
    Call {
        request: ProcessingRequest,
        phone_number: ArrayVec<Digit, 32>,
        call_generation: Generation,
    },

    ResetLink {
        request: ProcessingRequest,
        state: reset_link::State,
    },
    EndLink {
        request: ProcessingRequest,
        state: end_link::State,
    },
    RecoverLink {
        request: ProcessingRequest,
        state: recover_link::State,
    },
}

impl State {
    fn vblank(
        &mut self,
        transfer_length: TransferLength,
        timer: Timer,
    ) -> Result<(), request::Timeout> {
        match self {
            Self::Linking { request, .. } => request.vblank(transfer_length),
            Self::Linked { request, state, .. } => {
                if let Some(request) = request {
                    request.vblank(transfer_length)
                } else {
                    // Schedule a new request.
                    let (new_state, new_request) = state.request(timer, transfer_length);
                    *state = new_state;
                    *request = new_request;
                    Ok(())
                }
            }
            Self::WaitingForCall { request, state, .. } => {
                if let Some(request) = request {
                    request.vblank(transfer_length)
                } else {
                    // Schedule a new request.
                    let (new_state, new_request) = state.request(timer, transfer_length);
                    *state = new_state;
                    *request = new_request.map(ProcessingRequest::Current);
                    Ok(())
                }
            }
            Self::Call { request, .. } => request.vblank(transfer_length),
            Self::ResetLink { request, .. } => request.vblank(transfer_length),
            Self::EndLink { request, .. } => request.vblank(transfer_length),
            Self::RecoverLink { request, .. } => request.vblank(transfer_length),
        }
    }

    fn timer(&mut self, transfer_length: TransferLength) {
        match self {
            Self::Linking { request, .. } => request.timer(transfer_length),
            Self::Linked { request, .. } => {
                request
                    .as_mut()
                    .map(|request| request.timer(transfer_length));
            }
            Self::WaitingForCall { request, .. } => {
                request
                    .as_mut()
                    .map(|request| request.timer(transfer_length));
            }
            Self::Call { request, .. } => request.timer(transfer_length),
            Self::ResetLink { request, .. } => request.timer(transfer_length),
            Self::EndLink { request, .. } => request.timer(transfer_length),
            Self::RecoverLink { request, .. } => request.timer(transfer_length),
        }
    }

    /// Returning `Ok(Some(State))` means that we are still in an active state.
    ///
    /// Returning `Ok(None)` means that we are no longer in an active state (i.e. no longer
    /// connected).
    ///
    /// Returning `Err` means that we are no longer in an active state and the driver should enter
    /// an error state.
    fn serial(
        self,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
        timer: Timer,
    ) -> Result<Option<Self>, Either<request::Error, command::Error>> {
        match self {
            Self::Linking { request, state } => {
                match request.serial(adapter, transfer_length, timer) {
                    Ok(Some(next_request)) => Ok(Some(Self::Linking {
                        request: next_request,
                        state,
                    })),
                    Ok(None) => {
                        if let Some(new_state) = state.next() {
                            let new_request = new_state.request(timer, *transfer_length);
                            Ok(Some(Self::Linking {
                                request: new_request,
                                state: new_state,
                            }))
                        } else {
                            Ok(Some(Self::Linked {
                                request: None,
                                state: linked::State::new(),
                                call_generation: Generation::new(),
                                call_error: None,
                            }))
                        }
                    }
                    Err(error) => Err(error),
                }
            }
            Self::Linked {
                request,
                state,
                call_generation,
                call_error,
            } => {
                if let Some(request) = request {
                    request
                        .serial(adapter, transfer_length, timer)
                        .map(|request| {
                            Some(Self::Linked {
                                request,
                                state,
                                call_generation,
                                call_error,
                            })
                        })
                } else {
                    Ok(Some(Self::Linked {
                        request,
                        state,
                        call_generation,
                        call_error,
                    }))
                }
            }
            Self::WaitingForCall {
                request,
                state,
                call_generation,
            } => {
                match request {
                    Some(ProcessingRequest::Current(request)) => {
                        match request.serial(adapter, transfer_length, timer) {
                            Ok(Some(next_request)) => Ok(Some(Self::WaitingForCall {
                                request: Some(ProcessingRequest::Current(next_request)),
                                state,
                                call_generation,
                            })),
                            Ok(None) => todo!("connection established"),
                            Err(Either::Right(command::Error::WaitForTelephoneCall(
                                command::error::wait_for_telephone_call::Error::NoCallReceived,
                            ))) => {
                                // We retry on this specific command error.
                                Ok(Some(Self::WaitingForCall {
                                    request: None,
                                    state,
                                    call_generation,
                                }))
                            }
                            Err(Either::Left(request_error)) => Err(Either::Left(request_error)),
                            Err(Either::Right(command_error)) => Ok(Some(Self::Linked {
                                request: None,
                                state: linked::State::new(),
                                call_generation,
                                call_error: Some(command_error),
                            })),
                        }
                    }
                    Some(ProcessingRequest::Previous(request)) => request
                        .serial(adapter, transfer_length, timer)
                        .map(|request| {
                            Some(Self::WaitingForCall {
                                request: request.map(ProcessingRequest::Previous),
                                state,
                                call_generation,
                            })
                        })
                        .map_err(Either::Left),
                    None => Ok(Some(Self::WaitingForCall {
                        request,
                        state,
                        call_generation,
                    })),
                }
            }
            Self::Call {
                request,
                phone_number,
                call_generation,
            } => match request {
                ProcessingRequest::Current(request) => {
                    match request.serial(adapter, transfer_length, timer) {
                        Ok(Some(next_request)) => Ok(Some(Self::Call {
                            request: ProcessingRequest::Current(next_request),
                            phone_number,
                            call_generation,
                        })),
                        Ok(None) => todo!("connection esetablished"),
                        Err(Either::Left(request_error)) => Err(Either::Left(request_error)),
                        Err(Either::Right(command_error)) => Ok(Some(Self::Linked {
                            request: None,
                            state: linked::State::new(),
                            call_generation,
                            call_error: Some(command_error),
                        })),
                    }
                }
                ProcessingRequest::Previous(request) => {
                    match request.serial(adapter, transfer_length, timer) {
                        Ok(Some(next_request)) => Ok(Some(Self::Call {
                            request: ProcessingRequest::Previous(next_request),
                            phone_number,
                            call_generation,
                        })),
                        Ok(None) => Ok(Some(Self::Call {
                            request: ProcessingRequest::Current(Request::new_packet(
                                timer,
                                *transfer_length,
                                Source::Call {
                                    adapter: *adapter,
                                    phone_number: phone_number.clone(),
                                },
                            )),
                            phone_number,
                            call_generation,
                        })),
                        Err(error) => Err(Either::Left(error)),
                    }
                }
            },
            Self::ResetLink { request, state } => match request {
                ProcessingRequest::Current(request) => {
                    match request.serial(adapter, transfer_length, timer) {
                        Ok(Some(next_request)) => Ok(Some(Self::ResetLink {
                            request: ProcessingRequest::Current(next_request),
                            state,
                        })),
                        Ok(None) => {
                            if let Some(new_state) = state.next() {
                                let new_request = new_state.request(timer, *transfer_length);
                                Ok(Some(Self::ResetLink {
                                    request: ProcessingRequest::Current(new_request),
                                    state: new_state,
                                }))
                            } else {
                                Ok(Some(Self::Linked {
                                    request: None,
                                    state: linked::State::new(),
                                    call_generation: Generation::new(),
                                    call_error: None,
                                }))
                            }
                        }
                        Err(error) => Err(error),
                    }
                }
                ProcessingRequest::Previous(request) => {
                    match request.serial(adapter, transfer_length, timer) {
                        Ok(Some(next_request)) => Ok(Some(Self::ResetLink {
                            request: ProcessingRequest::Previous(next_request),
                            state,
                        })),
                        Ok(None) => Ok(Some(Self::ResetLink {
                            request: ProcessingRequest::Current(
                                state.request(timer, *transfer_length),
                            ),
                            state,
                        })),
                        Err(error) => Err(Either::Left(error)),
                    }
                }
            },
            Self::EndLink { request, state } => match request {
                ProcessingRequest::Current(request) => {
                    match request.serial(adapter, transfer_length, timer) {
                        Ok(Some(next_request)) => Ok(Some(Self::EndLink {
                            request: ProcessingRequest::Current(next_request),
                            state,
                        })),
                        Ok(None) => {
                            if let Some(new_state) = state.next() {
                                let new_request = new_state.request(timer, *transfer_length);
                                Ok(Some(Self::EndLink {
                                    request: ProcessingRequest::Current(new_request),
                                    state: new_state,
                                }))
                            } else {
                                Ok(None)
                            }
                        }
                        Err(error) => Err(error),
                    }
                }
                ProcessingRequest::Previous(request) => {
                    match request.serial(adapter, transfer_length, timer) {
                        Ok(Some(next_request)) => Ok(Some(Self::EndLink {
                            request: ProcessingRequest::Previous(next_request),
                            state,
                        })),
                        Ok(None) => Ok(Some(Self::EndLink {
                            request: ProcessingRequest::Current(
                                state.request(timer, *transfer_length),
                            ),
                            state,
                        })),
                        Err(error) => Err(Either::Left(error)),
                    }
                }
            },
            Self::RecoverLink { request, state } => match request {
                ProcessingRequest::Current(request) => {
                    match request.serial(adapter, transfer_length, timer) {
                        Ok(Some(next_request)) => Ok(Some(Self::RecoverLink {
                            request: ProcessingRequest::Current(next_request),
                            state,
                        })),
                        Ok(None) => {
                            if let Some(new_state) = state.next() {
                                let new_request = new_state.request(timer, *transfer_length);
                                Ok(Some(Self::RecoverLink {
                                    request: ProcessingRequest::Current(new_request),
                                    state: new_state,
                                }))
                            } else {
                                Ok(Some(Self::Linked {
                                    request: None,
                                    state: linked::State::new(),
                                    call_generation: Generation::new(),
                                    call_error: None,
                                }))
                            }
                        }
                        Err(error) => Err(error),
                    }
                }
                ProcessingRequest::Previous(request) => {
                    match request.serial(adapter, transfer_length, timer) {
                        Ok(Some(next_request)) => Ok(Some(Self::RecoverLink {
                            request: ProcessingRequest::Previous(next_request),
                            state,
                        })),
                        Ok(None) => Ok(Some(Self::RecoverLink {
                            request: ProcessingRequest::Current(
                                state.request(timer, *transfer_length),
                            ),
                            state,
                        })),
                        Err(error) => Err(Either::Left(error)),
                    }
                }
            },
        }
    }

    fn linking_status(&self) -> Result<bool, error::link::Error> {
        match self {
            Self::Linking { .. } => Ok(false),
            Self::Linked { .. } => Ok(true),
            Self::WaitingForCall { .. } => Ok(true),
            Self::Call { .. } => Ok(true),
            Self::ResetLink { .. } => Ok(false),
            Self::EndLink { .. } => Err(error::link::Error::closed()),
            Self::RecoverLink { .. } => Ok(false),
        }
    }

    fn p2p_status(&self, call_generation: Generation) -> Result<bool, error::p2p::Error> {
        match self {
            Self::Linking { .. } => Err(error::link::Error::superseded().into()),
            Self::Linked {
                call_generation: state_call_generation,
                call_error: Some(error),
                ..
            } if call_generation == *state_call_generation => Err(error.clone().into()),
            Self::Linked {
                call_generation: state_call_generation,
                call_error: None,
                ..
            } if call_generation == *state_call_generation => Err(error::p2p::Error::closed()),
            Self::Linked { .. } => Err(error::p2p::Error::superseded()),
            Self::WaitingForCall {
                call_generation: state_call_generation,
                ..
            } if call_generation == *state_call_generation => Ok(false),
            Self::WaitingForCall { .. } => Err(error::p2p::Error::superseded()),
            Self::Call {
                call_generation: state_call_generation,
                ..
            } if call_generation == *state_call_generation => Ok(false),
            Self::Call { .. } => Err(error::p2p::Error::superseded()),
            Self::ResetLink { .. } => Err(error::link::Error::superseded().into()),
            Self::EndLink { .. } => Err(error::link::Error::closed().into()),
            Self::RecoverLink { .. } => Err(error::link::Error::superseded().into()),
        }
    }

    fn reset_link(self, timer: Timer, transfer_length: TransferLength) -> Self {
        match self {
            // If we are already trying to establish a link, we simply continue without resetting.
            Self::Linking { request, state } => Self::Linking { request, state },
            Self::ResetLink { request, state } => Self::ResetLink { request, state },
            Self::RecoverLink { request, state } => Self::RecoverLink { request, state },

            // If we are currently ending the link, we can't stop it from happening. Therefore, we
            // begin the process of recovering the link.
            Self::EndLink {
                request:
                    ProcessingRequest::Current(request)
                    | ProcessingRequest::Previous(PreviousRequest::EndLink(request)),
                ..
            } => Self::RecoverLink {
                request: ProcessingRequest::Previous(PreviousRequest::EndLink(request)),
                state: recover_link::State::new(),
            },

            Self::Linked {
                request: Some(request),
                ..
            } => Self::ResetLink {
                request: ProcessingRequest::Previous(PreviousRequest::Linked(request)),
                state: reset_link::State::new(),
            },
            Self::WaitingForCall {
                request: Some(ProcessingRequest::Current(request)),
                ..
            } => Self::ResetLink {
                request: ProcessingRequest::Previous(PreviousRequest::WaitingForCall(request)),
                state: reset_link::State::new(),
            },
            Self::WaitingForCall {
                request: Some(ProcessingRequest::Previous(request)),
                ..
            } => Self::ResetLink {
                request: ProcessingRequest::Previous(request),
                state: reset_link::State::new(),
            },
            Self::Call {
                request: ProcessingRequest::Current(request),
                ..
            } => Self::ResetLink {
                request: ProcessingRequest::Previous(PreviousRequest::Call(request)),
                state: reset_link::State::new(),
            },
            Self::Call {
                request: ProcessingRequest::Previous(request),
                ..
            } => Self::ResetLink {
                request: ProcessingRequest::Previous(request),
                state: reset_link::State::new(),
            },
            Self::EndLink {
                request: ProcessingRequest::Previous(request),
                ..
            } => Self::ResetLink {
                request: ProcessingRequest::Previous(request),
                state: reset_link::State::new(),
            },

            Self::Linked { request: None, .. } | Self::WaitingForCall { request: None, .. } => {
                let reset_link_state = reset_link::State::new();
                Self::ResetLink {
                    request: ProcessingRequest::Current(
                        reset_link_state.request(timer, transfer_length),
                    ),
                    state: reset_link_state,
                }
            }
        }
    }

    fn end_link(self, timer: Timer, transfer_length: TransferLength) -> Self {
        match self {
            Self::Linking { request, .. } => Self::EndLink {
                request: ProcessingRequest::Previous(PreviousRequest::Linking(request)),
                state: end_link::State::new(),
            },
            Self::Linked {
                request: Some(request),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(PreviousRequest::Linked(request)),
                state: end_link::State::new(),
            },
            Self::WaitingForCall {
                request: Some(ProcessingRequest::Current(request)),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(PreviousRequest::WaitingForCall(request)),
                state: end_link::State::new(),
            },
            Self::WaitingForCall {
                request: Some(ProcessingRequest::Previous(request)),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(request),
                state: end_link::State::new(),
            },
            Self::Call {
                request: ProcessingRequest::Current(request),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(PreviousRequest::Call(request)),
                state: end_link::State::new(),
            },
            Self::Call {
                request: ProcessingRequest::Previous(request),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(request),
                state: end_link::State::new(),
            },
            Self::ResetLink {
                request: ProcessingRequest::Current(request),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(PreviousRequest::ResetLink(request)),
                state: end_link::State::new(),
            },
            Self::ResetLink {
                request: ProcessingRequest::Previous(request),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(request),
                state: end_link::State::new(),
            },
            Self::RecoverLink {
                request: ProcessingRequest::Current(request),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(PreviousRequest::RecoverLink(request)),
                state: end_link::State::new(),
            },
            Self::RecoverLink {
                request: ProcessingRequest::Previous(request),
                ..
            } => Self::EndLink {
                request: ProcessingRequest::Previous(request),
                state: end_link::State::new(),
            },

            Self::Linked { request: None, .. } | Self::WaitingForCall { request: None, .. } => {
                let end_link_state = end_link::State::new();
                Self::EndLink {
                    request: ProcessingRequest::Current(
                        end_link_state.request(timer, transfer_length),
                    ),
                    state: end_link_state,
                }
            }

            Self::EndLink { request, state } => Self::EndLink { request, state },
        }
    }

    fn wait_for_call(self) -> Result<(Self, Generation), error::link::Error> {
        match self {
            Self::Linking { .. }
            | Self::ResetLink { .. }
            | Self::EndLink { .. }
            | Self::RecoverLink { .. } => Err(error::link::Error::superseded()),
            Self::Linked {
                request: Some(request),
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::WaitingForCall {
                        request: Some(ProcessingRequest::Previous(PreviousRequest::Linked(
                            request,
                        ))),
                        state: waiting_for_call::State::new(),
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::Linked {
                request: None,
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::WaitingForCall {
                        request: None,
                        state: waiting_for_call::State::new(),
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::WaitingForCall {
                request: Some(ProcessingRequest::Current(request)),
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::WaitingForCall {
                        request: Some(ProcessingRequest::Previous(
                            PreviousRequest::WaitingForCall(request),
                        )),
                        state: waiting_for_call::State::new(),
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::WaitingForCall {
                request,
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::WaitingForCall {
                        request,
                        state: waiting_for_call::State::new(),
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::Call {
                request: ProcessingRequest::Current(request),
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::WaitingForCall {
                        request: Some(ProcessingRequest::Previous(PreviousRequest::Call(request))),
                        state: waiting_for_call::State::new(),
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::Call {
                request,
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::WaitingForCall {
                        request: Some(request),
                        state: waiting_for_call::State::new(),
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
        }
    }

    fn call(
        self,
        phone_number: ArrayVec<Digit, 32>,
        timer: Timer,
        transfer_length: TransferLength,
        adapter: Adapter,
    ) -> Result<(Self, Generation), error::link::Error> {
        match self {
            Self::Linking { .. }
            | Self::ResetLink { .. }
            | Self::EndLink { .. }
            | Self::RecoverLink { .. } => Err(error::link::Error::superseded()),
            Self::Linked {
                request: Some(request),
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::Call {
                        request: ProcessingRequest::Previous(PreviousRequest::Linked(request)),
                        phone_number,
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::Linked {
                request: None,
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::Call {
                        request: ProcessingRequest::Current(Request::new_packet(
                            timer,
                            transfer_length,
                            Source::Call {
                                adapter,
                                phone_number: phone_number.clone(),
                            },
                        )),
                        phone_number,
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::WaitingForCall {
                request: Some(ProcessingRequest::Current(request)),
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::Call {
                        request: ProcessingRequest::Previous(PreviousRequest::WaitingForCall(
                            request,
                        )),
                        phone_number,
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::WaitingForCall {
                request: Some(ProcessingRequest::Previous(request)),
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::Call {
                        request: ProcessingRequest::Previous(request),
                        phone_number,
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::WaitingForCall {
                request: None,
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::Call {
                        request: ProcessingRequest::Current(Request::new_packet(
                            timer,
                            transfer_length,
                            Source::Call {
                                adapter,
                                phone_number: phone_number.clone(),
                            },
                        )),
                        phone_number,
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::Call {
                request: ProcessingRequest::Current(request),
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::Call {
                        request: ProcessingRequest::Previous(PreviousRequest::Call(request)),
                        phone_number,
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
            Self::Call {
                request,
                call_generation,
                ..
            } => {
                let new_call_generation = call_generation.increment();
                Ok((
                    State::Call {
                        request,
                        phone_number,
                        call_generation: new_call_generation,
                    },
                    new_call_generation,
                ))
            }
        }
    }
}
