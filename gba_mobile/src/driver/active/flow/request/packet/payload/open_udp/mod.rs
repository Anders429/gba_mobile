pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::{driver::Command, socket};
use core::{net::SocketAddrV4, num::NonZeroU16};
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct OpenUdp {
    socket: SocketAddrV4,
}

impl OpenUdp {
    pub(in crate::driver::active::flow) fn new(socket: SocketAddrV4) -> Self {
        Self { socket }
    }
}

impl Payload for OpenUdp {
    type Send = Self;

    type ReceiveCommand = ReceiveCommand;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for OpenUdp {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        Command::OpenUdpConnection
    }

    fn length(&self) -> u8 {
        6
    }

    fn get(&self, index: u8) -> u8 {
        if let Some(port_index) = index.checked_sub(4) {
            self.socket
                .port()
                .to_be_bytes()
                .as_slice()
                .get(port_index as usize)
                .copied()
                .unwrap_or(0x00)
        } else {
            self.socket
                .ip()
                .octets()
                .get(index as usize)
                .copied()
                .unwrap_or(0x00)
        }
    }

    fn finish(self) -> Self::ReceiveCommand {
        ReceiveCommand
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveCommand;

impl super::ReceiveCommand for ReceiveCommand {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::OpenUdpConnection => Ok(ReceiveLength::OpenUdpConnection),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveLength {
    OpenUdpConnection,
    CommandError,
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
        match self {
            Self::OpenUdpConnection => {
                if length == 1 {
                    Ok(Either::Left(ReceiveData::OpenUdpConnection))
                } else {
                    Err((
                        error::InvalidLength::OpenUdpConnection(length),
                        ReceiveCommand,
                    ))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData::CommandError(
                        command_error::Data::new(),
                    )))
                } else {
                    Err((error::InvalidLength::CommandError(length), ReceiveCommand))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveData {
    OpenUdpConnection,
    CommandError(command_error::Data),
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
        match self {
            Self::OpenUdpConnection => Ok(Either::Right(ReceiveParsed { id: byte.into() })),
            Self::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::CommandError(data))),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    ReceiveCommand,
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    ReceiveCommand,
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveParsed {
    pub(in crate::driver::active::flow) id: socket::Id,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        Command::OpenUdpConnection
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand
    }
}
