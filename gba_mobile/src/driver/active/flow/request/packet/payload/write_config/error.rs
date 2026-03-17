use crate::driver::{Command, command};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) struct UnsupportedCommand(pub(super) Command);

impl Display for UnsupportedCommand {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "unsupported command {}; supported commands are {} and {}",
            self.0,
            Command::WriteConfigurationData,
            Command::CommandError
        )
    }
}

impl core::error::Error for UnsupportedCommand {}

#[derive(Clone, Debug)]
pub(in crate::driver) enum InvalidLength {
    WriteConfig(u8),
    CommandError(u8),
}

impl Display for InvalidLength {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::WriteConfig(length) => write!(
                formatter,
                "received length of {length} for {} packet, but expected length of 2",
                Command::WriteConfigurationData,
            ),
            Self::CommandError(length) => write!(
                formatter,
                "received length of {length} for {} packet, but expected length of 2",
                Command::CommandError
            ),
        }
    }
}

impl core::error::Error for InvalidLength {}

#[derive(Clone, Debug)]
pub(in crate::driver) enum InvalidData {
    FirstHalfOffset(u8),
    SecondHalfOffset(u8),
    InvalidLength(u8),
    UnknownCommandError(command::error::Unknown),
    UnexpectedCommandError(command::Error),
}

impl Display for InvalidData {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::FirstHalfOffset(offset) => write!(
                formatter,
                "received offset of {offset} when writing first half of config, but expected offset of 0"
            ),
            Self::SecondHalfOffset(offset) => write!(
                formatter,
                "received offset of {offset} when writing second half of config, but expected offset of 128"
            ),
            Self::InvalidLength(length) => write!(
                formatter,
                "received length of {length} when writing config, but expected length of 128"
            ),
            Self::UnknownCommandError(_) => {
                formatter.write_str("unable to parse command error payload")
            }
            Self::UnexpectedCommandError(_) => {
                formatter.write_str("received unexpected command error for beginning session")
            }
        }
    }
}

impl core::error::Error for InvalidData {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::FirstHalfOffset(_) => None,
            Self::SecondHalfOffset(_) => None,
            Self::InvalidLength(_) => None,
            Self::UnknownCommandError(unknown) => Some(unknown),
            Self::UnexpectedCommandError(error) => Some(error),
        }
    }
}
