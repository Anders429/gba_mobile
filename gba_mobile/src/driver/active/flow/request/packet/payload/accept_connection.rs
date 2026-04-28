use super::{super::Data, Error, Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct AcceptConnection {
    _private: PhantomData<()>,
}

impl AcceptConnection {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::WaitForTelephoneCall;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum Response {
    Connected,
    NotConnected,
}

impl Payload for AcceptConnection {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::WaitForTelephoneCall => {
                if data.data.is_empty() {
                    Ok(Response::Connected)
                } else {
                    Err(Error::InvalidLength {
                        command: Command::WaitForTelephoneCall,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::WaitForTelephoneCall(
                        command::error::wait_for_telephone_call::Error::NoCallReceived,
                    ) => Ok(Response::NotConnected),
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::WaitForTelephoneCall, Command::CommandError],
            }),
        }
    }
}
