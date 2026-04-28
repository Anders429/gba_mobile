use super::{super::Data, Error, Payload, command_error};
use crate::{ArrayVec, driver::Command, mmio::serial::TransferLength};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct EnableSio32 {
    _private: PhantomData<()>,
}

impl EnableSio32 {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::Sio32Mode;
        data.data = unsafe { ArrayVec::try_from_iter([0x01]).unwrap_unchecked() };

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for EnableSio32 {
    type Response<'a> = TransferLength;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::Sio32Mode => {
                if data.data.is_empty() {
                    Ok(TransferLength::_32Bit)
                } else {
                    Err(Error::InvalidLength {
                        command: Command::Sio32Mode,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::Reset => {
                if data.data.is_empty() {
                    Ok(TransferLength::_8Bit)
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
                expected: &[Command::Sio32Mode, Command::Reset, Command::CommandError],
            }),
        }
    }
}
