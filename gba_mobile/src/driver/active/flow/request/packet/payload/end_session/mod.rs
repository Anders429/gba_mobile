pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::Command;
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct EndSession;

impl Payload for EndSession {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = Self;
}

impl super::Send for EndSession {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::EndSession
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

impl super::ReceiveCommand for EndSession {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::EndSession => Ok(ReceiveLength::EndSession),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum ReceiveLength {
    EndSession,
    CommandError,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = EndSession;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = EndSession;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self {
            Self::EndSession => {
                if length == 0 {
                    Ok(Either::Right(EndSession))
                } else {
                    Err((error::InvalidLength::EndSession(length), EndSession))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData(command_error::Data::new())))
                } else {
                    Err((error::InvalidLength::CommandError(length), EndSession))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        EndSession
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) struct ReceiveData(command_error::Data);

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = EndSession;
    type ReceiveParsed = EndSession;
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
                EndSession,
                None,
            )),
            Err(error) => Err((
                error::InvalidData::UnknownCommandError(error),
                EndSession,
                None,
            )),
        }
    }
}

impl super::ReceiveParsed for EndSession {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::EndSession
    }

    fn restart(self) -> Self::ReceiveCommand {
        self
    }
}
