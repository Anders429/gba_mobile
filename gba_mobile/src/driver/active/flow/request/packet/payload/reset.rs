use super::{super::Data, Error, Payload, command_error};
use crate::{ArrayVec, driver::Command};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Reset {
    _private: PhantomData<()>,
}

impl Reset {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::Reset;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for Reset {
    type Response<'a> = ();
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::Reset => {
                if data.data.is_empty() {
                    Ok(())
                } else {
                    Err(Error::InvalidLength {
                        command: Command::Reset,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => command_error::parse(&data.data)
                .and_then(|error| Err(Error::UnexpectedCommandError(error))),
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::Reset, Command::CommandError],
            }),
        }
    }
}
