pub(in crate::driver) mod data;
pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::{config::format::Location, driver::Command};
use core::num::NonZeroU16;
use data::Data;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct ReadConfig {
    location: Location,
}

impl ReadConfig {
    pub(in crate::driver::active::flow) fn new(location: Location) -> Self {
        Self { location }
    }
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
            0 => self.location.offset,
            1 => self.location.length.get(),
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
                location: self.location,
            }),
            Command::CommandError => Ok(ReceiveLength {
                received_command: ReceivedCommand::CommandError,
                location: self.location,
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
    location: Location,
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
                if length == self.location.length.get() + 1 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::ReadConfig(Data::new()),
                        location: self.location,
                    }))
                } else {
                    Err((
                        error::InvalidLength::ReadConfig(length, self.location.length.get() + 1),
                        ReadConfig {
                            location: self.location,
                        },
                    ))
                }
            }
            ReceivedCommand::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::CommandError(command_error::Data::new()),
                        location: self.location,
                    }))
                } else {
                    Err((
                        error::InvalidLength::CommandError(length),
                        ReadConfig {
                            location: self.location,
                        },
                    ))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReadConfig {
            location: self.location,
        }
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
    location: Location,
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
                .receive_data(byte, self.location)
                .map(|data| match data {
                    Either::Left(data) => Either::Left(Self {
                        command_data: CommandData::ReadConfig(data),
                        location: self.location,
                    }),
                    Either::Right(data) => Either::Right(ReceiveParsed {
                        data,
                        location: self.location,
                    }),
                })
                .map_err(|(error, index)| {
                    (
                        error::InvalidData::ReadConfig(error),
                        ReadConfig {
                            location: self.location,
                        },
                        index.map(|index| (unsafe { NonZeroU16::new_unchecked(129) }, index)),
                    )
                }),
            CommandData::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self {
                    command_data: CommandData::CommandError(data),
                    location: self.location,
                })),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    ReadConfig {
                        location: self.location,
                    },
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    ReadConfig {
                        location: self.location,
                    },
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveParsed {
    data: [u8; 128],
    location: Location,
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
        ReadConfig {
            location: self.location,
        }
    }
}
