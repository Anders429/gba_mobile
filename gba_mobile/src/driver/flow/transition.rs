use crate::{ArrayVec, Generation, driver::flow::end_session, phone_number::Digit};

#[derive(Debug)]
pub(in crate::driver) enum Destination {
    WaitingForCall {
        call_generation: Generation,
    },
    Call {
        call_generation: Generation,
        phone_number: ArrayVec<Digit, 32>,
    },
    EndSession(end_session::Destination),
}
