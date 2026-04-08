pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::{
    driver::{Command, command},
    socket,
};
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct CloseTcp {
    id: socket::Id,
}

impl CloseTcp {
    pub(in crate::driver::active::flow) fn new(id: socket::Id) -> Self {
        Self { id }
    }
}

impl Payload for CloseTcp {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for CloseTcp {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::CloseTcpConnection
    }

    fn length(&self) -> u8 {
        1
    }

    fn get(&self, _index: u8) -> u8 {
        self.id.0
    }

    fn finish(self) -> Self::ReceiveCommand {
        self
    }
}

impl super::ReceiveCommand for CloseTcp {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::CloseTcpConnection => Ok(ReceiveLength {
                received_command: ReceivedCommand::CloseTcpConnection,
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
    CloseTcpConnection,
    CommandError,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveLength {
    received_command: ReceivedCommand,
    id: socket::Id,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = CloseTcp;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self.received_command {
            ReceivedCommand::CloseTcpConnection => {
                if length == 1 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::CloseTcpConnection,
                        id: self.id,
                    }))
                } else {
                    Err((
                        error::InvalidLength::CloseTcpConnection(length),
                        CloseTcp { id: self.id },
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
                        CloseTcp { id: self.id },
                    ))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        CloseTcp { id: self.id }
    }
}

#[derive(Debug)]
enum CommandData {
    CloseTcpConnection,
    CommandError(command_error::Data),
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData {
    command_data: CommandData,
    id: socket::Id,
}

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = CloseTcp;
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
            CommandData::CloseTcpConnection => {
                let received_id = byte.into();
                if self.id == received_id {
                    Ok(Either::Right(ReceiveParsed {
                        response: Response::Closed,
                        id: self.id,
                    }))
                } else {
                    Err((
                        error::InvalidData::InvalidSocketId {
                            expected: self.id,
                            received: received_id,
                        },
                        CloseTcp { id: self.id },
                        None,
                    ))
                }
            }
            CommandData::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self {
                    command_data: CommandData::CommandError(data),
                    id: self.id,
                })),
                Ok(Either::Right(command::Error::CloseTcpConnection(
                    command::error::close_tcp_connection::Error::NotConnected,
                ))) => Ok(Either::Right(ReceiveParsed {
                    response: Response::AlreadyClosed,
                    id: self.id,
                })),
                Ok(Either::Right(command::Error::CloseTcpConnection(
                    command::error::close_tcp_connection::Error::NotLoggedIn,
                ))) => Ok(Either::Right(ReceiveParsed {
                    response: Response::AlreadyDisconnected,
                    id: self.id,
                })),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    CloseTcp { id: self.id },
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    CloseTcp { id: self.id },
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum Response {
    Closed,
    AlreadyClosed,
    AlreadyDisconnected,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveParsed {
    pub(in crate::driver::active::flow) response: Response,
    id: socket::Id,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = CloseTcp;

    fn command(&self) -> Command {
        match self.response {
            Response::Closed => Command::CloseTcpConnection,
            Response::AlreadyClosed => Command::CommandError,
            Response::AlreadyDisconnected => Command::CommandError,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        CloseTcp { id: self.id }
    }
}
