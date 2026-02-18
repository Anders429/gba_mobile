use crate::{
    PhoneNumber, Timer,
    driver::{Adapter, Request, Source},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
enum Flow {
    PreviousRequest,
    Call,
}

#[derive(Clone, Debug)]
pub(in crate::driver) struct Call {
    flow: Flow,
    phone_number: PhoneNumber,
}

impl Call {
    pub(in crate::driver) fn new(phone_number: PhoneNumber) -> Self {
        Self {
            flow: Flow::PreviousRequest,
            phone_number,
        }
    }

    pub(in crate::driver) fn request(
        &self,
        timer: Timer,
        transfer_length: TransferLength,
        adapter: Adapter,
    ) -> Request {
        match self.flow {
            // If we didn't have a previous request, we just send a single idle byte.
            Flow::PreviousRequest => Request::new_idle(timer, transfer_length),
            Flow::Call => Request::new_packet(
                timer,
                transfer_length,
                Source::Call {
                    adapter,
                    phone_number: self.phone_number.clone(),
                },
            ),
        }
    }

    pub(in crate::driver) fn next(self) -> Option<Self> {
        match self.flow {
            Flow::PreviousRequest => Some(Self {
                flow: Flow::Call,
                phone_number: self.phone_number,
            }),
            Flow::Call => None,
        }
    }
}
