use super::{super::Data, Error, Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Disconnect {
    _private: PhantomData<()>,
}

impl Disconnect {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::HangUpTelephone;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for Disconnect {
    type Response<'a> = ();
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::HangUpTelephone => {
                if data.data.is_empty() {
                    Ok(())
                } else {
                    Err(Error::InvalidLength {
                        command: Command::HangUpTelephone,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::HangUpTelephone(
                        command::error::hang_up_telephone::Error::NotConnected,
                    ) => Ok(()),
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::HangUpTelephone, Command::CommandError],
            }),
        }
    }
}
