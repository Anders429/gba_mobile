pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::Command;
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct Reset;

impl Payload for Reset {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = Self;
}

impl super::Send for Reset {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::Reset
    }

    fn length(&self) -> u8 {
        0
    }

    fn get(&self, _index: u8) -> u8 {
        0x00
    }

    fn finish(self) -> Self::ReceiveCommand {
        self
    }
}

impl super::ReceiveCommand for Reset {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::Reset => Ok(ReceiveLength::Reset),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveLength {
    Reset,
    CommandError,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = Reset;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = Reset;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self {
            Self::Reset => {
                if length == 0 {
                    Ok(Either::Right(Reset))
                } else {
                    Err((error::InvalidLength::Reset(length), Reset))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData(command_error::Data::new())))
                } else {
                    Err((error::InvalidLength::CommandError(length), Reset))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        Reset
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData(command_error::Data);

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = Reset;
    type ReceiveParsed = Reset;
    type Error = error::InvalidData;

    fn receive_data(
        self,
        byte: u8,
    ) -> Result<
        Either<Self, Self::ReceiveParsed>,
        (Self::Error, Self::ReceiveCommand, Option<(NonZeroU16, u16)>),
    > {
        match self.0.receive_data(byte) {
            Ok(Either::Left(data)) => Ok(Either::Left(Self(data))),
            Ok(Either::Right(command_error)) => Err((
                error::InvalidData::UnexpectedCommandError(command_error),
                Reset,
                None,
            )),
            Err(error) => Err((error::InvalidData::UnknownCommandError(error), Reset, None)),
        }
    }
}

impl super::ReceiveParsed for Reset {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::Reset
    }

    fn restart(self) -> Self::ReceiveCommand {
        self
    }
}
