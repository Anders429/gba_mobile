pub(in crate::driver) mod error;

mod data;

use super::{Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
    socket,
};
use core::num::NonZeroU16;
use data::Data;
use deranged::RangedU8;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct TransferData {
    id: socket::Id,
    data: ArrayVec<u8, 254>,
}

impl TransferData {
    pub(in crate::driver::active::flow) fn new(id: socket::Id, data: ArrayVec<u8, 254>) -> Self {
        Self { id, data }
    }
}

impl Payload for TransferData {
    type Send = Self;

    type ReceiveCommand = ReceiveCommand;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for TransferData {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        Command::TransferData
    }

    fn length(&self) -> u8 {
        self.data.len() + 1
    }

    fn get(&self, index: u8) -> u8 {
        if let Some(data_index) = index.checked_sub(1) {
            self.data.get(data_index).copied().unwrap_or(0x00)
        } else {
            self.id.0
        }
    }

    fn finish(self) -> Self::ReceiveCommand {
        ReceiveCommand { id: self.id }
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveCommand {
    id: socket::Id,
}

impl super::ReceiveCommand for ReceiveCommand {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::TransferData => Ok(ReceiveLength {
                received_command: ReceivedCommand::TransferData,
                id: self.id,
            }),
            Command::ConnectionClosed => Ok(ReceiveLength {
                received_command: ReceivedCommand::ConnectionClosed,
                id: self.id,
            }),
            Command::CommandError => Ok(ReceiveLength {
                received_command: ReceivedCommand::CommandError,
                id: self.id,
            }),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
enum ReceivedCommand {
    TransferData,
    ConnectionClosed,
    CommandError,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveLength {
    received_command: ReceivedCommand,
    id: socket::Id,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = ReceiveCommand;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self.received_command {
            ReceivedCommand::TransferData => {
                if length > 0 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::TransferData(Data::new(self.id, unsafe {
                            RangedU8::new_unchecked(length - 1)
                        })),
                        id: self.id,
                    }))
                } else {
                    Err((
                        error::InvalidLength::TransferData,
                        ReceiveCommand { id: self.id },
                    ))
                }
            }
            ReceivedCommand::ConnectionClosed => {
                if length > 0 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::ConnectionClosed(Data::new(self.id, unsafe {
                            RangedU8::new_unchecked(length - 1)
                        })),
                        id: self.id,
                    }))
                } else {
                    Err((
                        error::InvalidLength::ConnectionClosed,
                        ReceiveCommand { id: self.id },
                    ))
                }
            }
            ReceivedCommand::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::CommandError(command_error::Data::new()),
                        id: self.id,
                    }))
                } else {
                    Err((
                        error::InvalidLength::CommandError(length),
                        ReceiveCommand { id: self.id },
                    ))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand { id: self.id }
    }
}

#[derive(Debug)]
enum CommandData {
    TransferData(Data),
    ConnectionClosed(Data),
    CommandError(command_error::Data),
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData {
    command_data: CommandData,
    id: socket::Id,
}

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = ReceiveCommand;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidData;

    fn receive_data(
        self,
        byte: u8,
    ) -> Result<
        Either<Self, Self::ReceiveParsed>,
        (Self::Error, Self::ReceiveCommand, Option<(NonZeroU16, u16)>),
    > {
        match self.command_data {
            CommandData::TransferData(data) => data
                .receive_data(byte)
                .map(|data| match data {
                    Either::Left(data) => Either::Left(Self {
                        command_data: CommandData::TransferData(data),
                        id: self.id,
                    }),
                    Either::Right(data) => Either::Right(ReceiveParsed {
                        response: Response::Data(data),
                        id: self.id,
                    }),
                })
                .map_err(|(error, index)| (error, ReceiveCommand { id: self.id }, index)),
            CommandData::ConnectionClosed(data) => data
                .receive_data(byte)
                .map(|data| match data {
                    Either::Left(data) => Either::Left(Self {
                        command_data: CommandData::ConnectionClosed(data),
                        id: self.id,
                    }),
                    Either::Right(data) => Either::Right(ReceiveParsed {
                        response: Response::FinalData(data),
                        id: self.id,
                    }),
                })
                .map_err(|(error, index)| (error, ReceiveCommand { id: self.id }, index)),
            CommandData::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self {
                    command_data: CommandData::CommandError(data),
                    id: self.id,
                })),
                Ok(Either::Right(command::Error::TransferData(
                    command::error::transfer_data::Error::CommunicationFailed,
                ))) => Ok(Either::Right(ReceiveParsed {
                    response: Response::ConnectionFailed,
                    id: self.id,
                })),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    ReceiveCommand { id: self.id },
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    ReceiveCommand { id: self.id },
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum Response {
    Data(ArrayVec<u8, 254>),
    FinalData(ArrayVec<u8, 254>),
    ConnectionFailed,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveParsed {
    pub(in crate::driver::active::flow) response: Response,
    id: socket::Id,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        match self.response {
            Response::Data(_) => Command::TransferData,
            Response::FinalData(_) => Command::ConnectionClosed,
            Response::ConnectionFailed => Command::CommandError,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand { id: self.id }
    }
}
