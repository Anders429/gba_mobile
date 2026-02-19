use crate::{Generation, PhoneNumber, driver::flow::end_session};

#[derive(Debug)]
pub(in crate::driver) enum Destination {
    WaitingForCall {
        call_generation: Generation,
    },
    Call {
        call_generation: Generation,
        phone_number: PhoneNumber,
    },
    EndSession(end_session::Destination),
}
