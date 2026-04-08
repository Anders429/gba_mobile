pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::{
    driver::{Command, command},
    socket,
};
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct CloseUdp {
    id: socket::Id,
}

impl CloseUdp {
    pub(in crate::driver::active::flow) fn new(id: socket::Id) -> Self {
        Self { id }
    }
}

impl Payload for CloseUdp {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for CloseUdp {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::CloseUdpConnection
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

impl super::ReceiveCommand for CloseUdp {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::CloseUdpConnection => Ok(ReceiveLength {
                received_command: ReceivedCommand::CloseUdpConnection,
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
    CloseUdpConnection,
    CommandError,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveLength {
    received_command: ReceivedCommand,
    id: socket::Id,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = CloseUdp;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self.received_command {
            ReceivedCommand::CloseUdpConnection => {
                if length == 1 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::CloseUdpConnection,
                        id: self.id,
                    }))
                } else {
                    Err((
                        error::InvalidLength::CloseUdpConnection(length),
                        CloseUdp { id: self.id },
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
                        CloseUdp { id: self.id },
                    ))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        CloseUdp { id: self.id }
    }
}

#[derive(Debug)]
enum CommandData {
    CloseUdpConnection,
    CommandError(command_error::Data),
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData {
    command_data: CommandData,
    id: socket::Id,
}

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = CloseUdp;
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
            CommandData::CloseUdpConnection => {
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
                        CloseUdp { id: self.id },
                        None,
                    ))
                }
            }
            CommandData::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self {
                    command_data: CommandData::CommandError(data),
                    id: self.id,
                })),
                Ok(Either::Right(command::Error::CloseUdpConnection(
                    command::error::close_udp_connection::Error::NotConnected,
                ))) => Ok(Either::Right(ReceiveParsed {
                    response: Response::AlreadyClosed,
                    id: self.id,
                })),
                Ok(Either::Right(command::Error::CloseUdpConnection(
                    command::error::close_udp_connection::Error::NotLoggedIn,
                ))) => Ok(Either::Right(ReceiveParsed {
                    response: Response::AlreadyDisconnected,
                    id: self.id,
                })),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    CloseUdp { id: self.id },
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    CloseUdp { id: self.id },
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
    type ReceiveCommand = CloseUdp;

    fn command(&self) -> Command {
        match self.response {
            Response::Closed => Command::CloseUdpConnection,
            Response::AlreadyClosed => Command::CommandError,
            Response::AlreadyDisconnected => Command::CommandError,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        CloseUdp { id: self.id }
    }
}
