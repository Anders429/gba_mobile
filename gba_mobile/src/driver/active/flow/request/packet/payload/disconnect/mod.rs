pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::Command;
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct Disconnect;

impl Payload for Disconnect {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = Self;
}

impl super::Send for Disconnect {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::HangUpTelephone
    }

    fn length(&self) -> u8 {
        0
    }

    fn get(&self, index: u8) -> u8 {
        0x00
    }

    fn finish(self) -> Self::ReceiveCommand {
        self
    }
}

impl super::ReceiveCommand for Disconnect {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::HangUpTelephone => Ok(ReceiveLength::HangUpTelephone),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveLength {
    HangUpTelephone,
    CommandError,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = Disconnect;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = Disconnect;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self {
            Self::HangUpTelephone => {
                if length == 0 {
                    Ok(Either::Right(Disconnect))
                } else {
                    Err((error::InvalidLength::HangUpTelephone(length), Disconnect))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData(command_error::Data::new())))
                } else {
                    Err((error::InvalidLength::CommandError(length), Disconnect))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        Disconnect
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData(command_error::Data);

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = Disconnect;
    type ReceiveParsed = Disconnect;
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
                Disconnect,
                None,
            )),
            Err(error) => Err((
                error::InvalidData::UnknownCommandError(error),
                Disconnect,
                None,
            )),
        }
    }
}

impl super::ReceiveParsed for Disconnect {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::HangUpTelephone
    }

    fn restart(self) -> Self::ReceiveCommand {
        self
    }
}
