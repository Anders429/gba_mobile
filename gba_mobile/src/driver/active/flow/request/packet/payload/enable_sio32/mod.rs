pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::Command;
use either::Either;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct EnableSio32;

impl Payload for EnableSio32 {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for EnableSio32 {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::Sio32Mode
    }

    fn length(&self) -> u8 {
        1
    }

    fn get(&self, _index: u8) -> u8 {
        0x01
    }

    fn finish(self) -> Self::ReceiveCommand {
        self
    }
}

impl super::ReceiveCommand for EnableSio32 {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::Sio32Mode => Ok(ReceiveLength::EnableSio32),
            Command::Reset => Ok(ReceiveLength::DisableSio32),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum ReceiveLength {
    EnableSio32,
    DisableSio32,
    CommandError,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = EnableSio32;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self {
            Self::EnableSio32 => {
                if length == 0 {
                    Ok(Either::Right(ReceiveParsed::EnableSio32))
                } else {
                    Err((error::InvalidLength::EnableSio32(length), EnableSio32))
                }
            }
            Self::DisableSio32 => {
                if length == 0 {
                    Ok(Either::Right(ReceiveParsed::DisableSio32))
                } else {
                    Err((error::InvalidLength::DisableSio32(length), EnableSio32))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData(command_error::Data::new())))
                } else {
                    Err((error::InvalidLength::CommandError(length), EnableSio32))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        EnableSio32
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) struct ReceiveData(command_error::Data);

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = EnableSio32;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidData;

    fn receive_data(
        self,
        byte: u8,
    ) -> Result<
        Either<Self, Self::ReceiveParsed>,
        (
            Self::Error,
            Self::ReceiveCommand,
            Option<(core::num::NonZeroU16, u16)>,
        ),
    > {
        match self.0.receive_data(byte) {
            Ok(Either::Left(data)) => Ok(Either::Left(Self(data))),
            Ok(Either::Right(command_error)) => Err((
                error::InvalidData::UnexpectedCommandError(command_error),
                EnableSio32,
                None,
            )),
            Err(error) => Err((
                error::InvalidData::UnknownCommandError(error),
                EnableSio32,
                None,
            )),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum ReceiveParsed {
    EnableSio32,
    DisableSio32,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = EnableSio32;

    fn command(&self) -> Command {
        match self {
            Self::EnableSio32 => Command::Sio32Mode,
            Self::DisableSio32 => Command::Reset,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        EnableSio32
    }
}
