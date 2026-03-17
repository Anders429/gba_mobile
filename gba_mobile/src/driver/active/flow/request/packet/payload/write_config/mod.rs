pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::driver::Command;
use core::num::NonZeroU16;
use either::Either;

#[derive(Clone, Copy, Debug)]
pub(in crate::driver::active::flow) enum Location {
    FirstHalf,
    SecondHalf,
}

#[derive(Debug)]
pub(in crate::driver) struct WriteConfig {
    location: Location,
    data: [u8; 128],
}

impl WriteConfig {
    pub(in crate::driver::active::flow) fn new(location: Location, data: [u8; 128]) -> Self {
        Self { location, data }
    }
}

impl Payload for WriteConfig {
    type Send = Self;

    type ReceiveCommand = ReceiveCommand;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for WriteConfig {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        Command::WriteConfigurationData
    }

    fn length(&self) -> u8 {
        129
    }

    fn get(&self, index: u8) -> u8 {
        if let Some(config_index) = index.checked_sub(1) {
            // Starting at index 1, we return the config bytes.
            self.data
                .get(config_index as usize)
                .copied()
                .unwrap_or(0x00)
        } else {
            // If this is the first byte, send the write location offset.
            match self.location {
                Location::FirstHalf => 0,
                Location::SecondHalf => 128,
            }
        }
    }

    fn finish(self) -> Self::ReceiveCommand {
        ReceiveCommand {
            location: self.location,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveCommand {
    location: Location,
}

impl super::ReceiveCommand for ReceiveCommand {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::WriteConfigurationData => Ok(ReceiveLength {
                received_command: ReceivedCommand::WriteConfig,
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
    WriteConfig,
    CommandError,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveLength {
    received_command: ReceivedCommand,
    location: Location,
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
            ReceivedCommand::WriteConfig => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::Offset,
                        location: self.location,
                    }))
                } else {
                    Err((
                        error::InvalidLength::WriteConfig(length),
                        ReceiveCommand {
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
                        ReceiveCommand {
                            location: self.location,
                        },
                    ))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand {
            location: self.location,
        }
    }
}

#[derive(Debug)]
enum CommandData {
    Offset,
    Length,
    CommandError(command_error::Data),
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData {
    command_data: CommandData,
    location: Location,
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
            CommandData::Offset => match self.location {
                Location::FirstHalf => {
                    if byte == 0 {
                        Ok(Either::Left(Self {
                            command_data: CommandData::Length,
                            location: self.location,
                        }))
                    } else {
                        Err((
                            error::InvalidData::FirstHalfOffset(byte),
                            ReceiveCommand {
                                location: self.location,
                            },
                            Some((unsafe { NonZeroU16::new_unchecked(2) }, 1)),
                        ))
                    }
                }
                Location::SecondHalf => {
                    if byte == 128 {
                        Ok(Either::Left(Self {
                            command_data: CommandData::Length,
                            location: self.location,
                        }))
                    } else {
                        Err((
                            error::InvalidData::SecondHalfOffset(byte),
                            ReceiveCommand {
                                location: self.location,
                            },
                            Some((unsafe { NonZeroU16::new_unchecked(2) }, 1)),
                        ))
                    }
                }
            },
            CommandData::Length => {
                if byte == 128 {
                    Ok(Either::Right(ReceiveParsed {
                        location: self.location,
                    }))
                } else {
                    Err((
                        error::InvalidData::InvalidLength(byte),
                        ReceiveCommand {
                            location: self.location,
                        },
                        None,
                    ))
                }
            }
            CommandData::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self {
                    command_data: CommandData::CommandError(data),
                    location: self.location,
                })),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    ReceiveCommand {
                        location: self.location,
                    },
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    ReceiveCommand {
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
    location: Location,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        Command::WriteConfigurationData
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand {
            location: self.location,
        }
    }
}
