use super::{super::Data, Error, Payload, command_error};
use crate::{ArrayVec, driver::Command};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct EndSession {
    _private: PhantomData<()>,
}

impl EndSession {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::EndSession;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for EndSession {
    type Response<'a> = ();
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::EndSession => {
                if data.data.is_empty() {
                    Ok(())
                } else {
                    Err(Error::InvalidLength {
                        command: Command::EndSession,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => command_error::parse(&data.data)
                .and_then(|error| Err(Error::UnexpectedCommandError(error))),
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::EndSession, Command::CommandError],
            }),
        }
    }
}
