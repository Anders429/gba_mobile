pub(in crate::driver) mod data;
pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::Command;
use core::num::NonZeroU16;
use data::Data;
use either::Either;

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) enum ReadConfig {
    FirstHalf,
    SecondHalf,
}

impl Payload for ReadConfig {
    type Send = Self;

    type ReceiveCommand = Self;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for ReadConfig {
    type ReceiveCommand = Self;

    fn command(&self) -> Command {
        Command::ReadConfigurationData
    }

    fn length(&self) -> u8 {
        2
    }

    fn get(&self, index: u8) -> u8 {
        match index {
            0 => match self {
                Self::FirstHalf => 0,
                Self::SecondHalf => 128,
            },
            1 => 128,
            _ => 0x00,
        }
    }

    fn finish(self) -> Self::ReceiveCommand {
        self
    }
}

impl super::ReceiveCommand for ReadConfig {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::ReadConfigurationData => Ok(ReceiveLength {
                received_command: ReceivedCommand::ReadConfig,
                read_config: self,
            }),
            Command::CommandError => Ok(ReceiveLength {
                received_command: ReceivedCommand::CommandError,
                read_config: self,
            }),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
enum ReceivedCommand {
    ReadConfig,
    CommandError,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveLength {
    received_command: ReceivedCommand,
    read_config: ReadConfig,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = ReadConfig;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self.received_command {
            ReceivedCommand::ReadConfig => {
                if length == 129 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::ReadConfig(Data::new()),
                        read_config: self.read_config,
                    }))
                } else {
                    Err((error::InvalidLength::ReadConfig(length), self.read_config))
                }
            }
            ReceivedCommand::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::CommandError(command_error::Data::new()),
                        read_config: self.read_config,
                    }))
                } else {
                    Err((error::InvalidLength::CommandError(length), self.read_config))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        self.read_config
    }
}

#[derive(Debug)]
enum CommandData {
    ReadConfig(Data),
    CommandError(command_error::Data),
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData {
    command_data: CommandData,
    read_config: ReadConfig,
}

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = ReadConfig;
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
            CommandData::ReadConfig(data) => data
                .receive_data(byte, self.read_config)
                .map(|data| match data {
                    Either::Left(data) => Either::Left(Self {
                        command_data: CommandData::ReadConfig(data),
                        read_config: self.read_config,
                    }),
                    Either::Right(data) => Either::Right(ReceiveParsed {
                        data,
                        read_config: self.read_config,
                    }),
                })
                .map_err(|(error, index)| {
                    (
                        error::InvalidData::ReadConfig(error),
                        self.read_config,
                        index.map(|index| (unsafe { NonZeroU16::new_unchecked(129) }, index)),
                    )
                }),
            CommandData::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self {
                    command_data: CommandData::CommandError(data),
                    read_config: self.read_config,
                })),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    self.read_config,
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    self.read_config,
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveParsed {
    data: [u8; 128],
    read_config: ReadConfig,
}

impl ReceiveParsed {
    pub(in crate::driver::active::flow) fn data(self) -> [u8; 128] {
        self.data
    }
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = ReadConfig;

    fn command(&self) -> Command {
        Command::ReadConfigurationData
    }

    fn restart(self) -> Self::ReceiveCommand {
        self.read_config
    }
}
