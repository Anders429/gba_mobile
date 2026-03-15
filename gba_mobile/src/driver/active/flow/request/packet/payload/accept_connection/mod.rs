pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::{Command, command};
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct AcceptConnection;

impl Payload for AcceptConnection {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for AcceptConnection {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::WaitForTelephoneCall
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

impl super::ReceiveCommand for AcceptConnection {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::WaitForTelephoneCall => Ok(ReceiveLength::WaitForTelephoneCall),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveLength {
    WaitForTelephoneCall,
    CommandError,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = AcceptConnection;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self {
            Self::WaitForTelephoneCall => {
                if length == 0 {
                    Ok(Either::Right(ReceiveParsed::Connected))
                } else {
                    Err((
                        error::InvalidLength::WaitForTelephoneCall(length),
                        AcceptConnection,
                    ))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData(command_error::Data::new())))
                } else {
                    Err((error::InvalidLength::CommandError(length), AcceptConnection))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        AcceptConnection
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData(command_error::Data);

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = AcceptConnection;
    type ReceiveParsed = ReceiveParsed;
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
            Ok(Either::Right(command::Error::WaitForTelephoneCall(
                command::error::wait_for_telephone_call::Error::NoCallReceived,
            ))) => Ok(Either::Right(ReceiveParsed::NotConnected)),
            Ok(Either::Right(command_error)) => Err((
                error::InvalidData::UnexpectedCommandError(command_error),
                AcceptConnection,
                None,
            )),
            Err(error) => Err((
                error::InvalidData::UnknownCommandError(error),
                AcceptConnection,
                None,
            )),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveParsed {
    Connected,
    NotConnected,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = AcceptConnection;

    fn command(&self) -> Command {
        match self {
            Self::Connected => Command::WaitForTelephoneCall,
            Self::NotConnected => Command::CommandError,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        AcceptConnection
    }
}
