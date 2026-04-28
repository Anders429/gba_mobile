use super::{super::Data, Error, Payload, command_error};
use crate::{
    Adapter, ArrayVec, Digit,
    driver::{Command, command},
};
use core::{iter, marker::PhantomData};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Connect {
    _private: PhantomData<()>,
}

impl Connect {
    pub(in crate::driver::active::flow) fn new(
        data: &mut Data,
        adapter: Adapter,
        digits: &ArrayVec<Digit, 32>,
    ) -> Self {
        data.command = Command::DialTelephone;
        data.data = unsafe {
            ArrayVec::try_from_iter(
                iter::once(adapter.dial_byte()).chain(digits.iter().map(|&digit| digit.into())),
            )
            .unwrap_unchecked()
        };

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

impl Payload for Connect {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::DialTelephone => {
                if data.data.is_empty() {
                    Ok(Response::Connected)
                } else {
                    Err(Error::InvalidLength {
                        command: Command::DialTelephone,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::DialTelephone(
                        command::error::dial_telephone::Error::LineBusy
                        | command::error::dial_telephone::Error::CommunicationFailed
                        | command::error::dial_telephone::Error::CallNotEstablished,
                    ) => Ok(Response::NotConnected),
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::DialTelephone, Command::CommandError],
            }),
        }
    }
}
