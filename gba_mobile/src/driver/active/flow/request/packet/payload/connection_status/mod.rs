pub(in crate::driver) mod data;
pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::Command;
use core::num::NonZeroU16;
use data::{Data, Status};
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct ConnectionStatus;

impl Payload for ConnectionStatus {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for ConnectionStatus {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::TelephoneStatus
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

impl super::ReceiveCommand for ConnectionStatus {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::TelephoneStatus => Ok(ReceiveLength::TelephoneStatus),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveLength {
    TelephoneStatus,
    CommandError,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = ConnectionStatus;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self {
            Self::TelephoneStatus => {
                if length == 3 {
                    Ok(Either::Left(ReceiveData::TelephoneStatus(Data::new())))
                } else {
                    Err((
                        error::InvalidLength::TelephoneStatus(length),
                        ConnectionStatus,
                    ))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData::CommandError(
                        command_error::Data::new(),
                    )))
                } else {
                    Err((error::InvalidLength::CommandError(length), ConnectionStatus))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ConnectionStatus
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveData {
    TelephoneStatus(Data),
    CommandError(command_error::Data),
}

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = ConnectionStatus;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidData;

    fn receive_data(
        self,
        byte: u8,
    ) -> Result<
        Either<Self, Self::ReceiveParsed>,
        (Self::Error, Self::ReceiveCommand, Option<(NonZeroU16, u16)>),
    > {
        match self {
            Self::TelephoneStatus(data) => data
                .receive_data(byte)
                .map(|data| {
                    data.map_left(Self::TelephoneStatus)
                        .map_right(|status| match status {
                            Status::Idle | Status::CallAvailable => ReceiveParsed::NotConnected,
                            Status::IncomingCall | Status::OutgoingCall => ReceiveParsed::Connected,
                        })
                })
                .map_err(|error| {
                    (
                        error::InvalidData::TelephoneStatus(error),
                        ConnectionStatus,
                        Some((unsafe { NonZeroU16::new_unchecked(3) }, 1)),
                    )
                }),
            Self::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::CommandError(data))),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    ConnectionStatus,
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    ConnectionStatus,
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveParsed {
    Connected,
    NotConnected,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = ConnectionStatus;

    fn command(&self) -> Command {
        Command::TelephoneStatus
    }

    fn restart(self) -> Self::ReceiveCommand {
        ConnectionStatus
    }
}
