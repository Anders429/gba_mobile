pub(in crate::driver) mod data;
pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::{Command, command};
use core::num::NonZeroU16;
use data::Data;
use either::Either;

const HANDSHAKE: [u8; 8] = [0x4e, 0x49, 0x4e, 0x54, 0x45, 0x4e, 0x44, 0x4f];

#[derive(Debug)]
pub(in crate::driver) struct BeginSession;

impl Payload for BeginSession {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for BeginSession {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::BeginSession
    }

    fn length(&self) -> u8 {
        HANDSHAKE.len() as u8
    }

    fn get(&self, index: u8) -> u8 {
        HANDSHAKE.get(index as usize).copied().unwrap_or(0x00)
    }

    fn finish(self) -> Self::ReceiveCommand {
        self
    }
}

impl super::ReceiveCommand for BeginSession {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::BeginSession => Ok(ReceiveLength::BeginSession),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveLength {
    BeginSession,
    CommandError,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = BeginSession;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self {
            Self::BeginSession => {
                if length == HANDSHAKE.len() as u8 {
                    Ok(Either::Left(ReceiveData::BeginSession(Data::new())))
                } else {
                    Err((error::InvalidLength::BeginSession(length), BeginSession))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData::CommandError(
                        command_error::Data::new(),
                    )))
                } else {
                    Err((error::InvalidLength::CommandError(length), BeginSession))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        BeginSession
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveData {
    BeginSession(Data),
    CommandError(command_error::Data),
}

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = BeginSession;
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
            Self::BeginSession(data) => data
                .receive_data(byte)
                .map(|data| match data {
                    Some(data) => Either::Left(Self::BeginSession(data)),
                    None => Either::Right(ReceiveParsed::BeginSession),
                })
                .map_err(|(error, index)| {
                    (
                        error::InvalidData::BeginSession(error),
                        BeginSession,
                        index.map(|index| (unsafe { NonZeroU16::new_unchecked(8) }, index)),
                    )
                }),

            Self::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::CommandError(data))),
                Ok(Either::Right(command::Error::BeginSession(
                    command::error::begin_session::Error::AlreadyActive,
                ))) => Ok(Either::Right(ReceiveParsed::AlreadyActive)),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    BeginSession,
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    BeginSession,
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveParsed {
    BeginSession,
    AlreadyActive,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = BeginSession;

    fn command(&self) -> Command {
        match self {
            Self::BeginSession => Command::BeginSession,
            Self::AlreadyActive => Command::CommandError,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        BeginSession
    }
}
