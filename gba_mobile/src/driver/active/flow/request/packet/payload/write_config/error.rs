use crate::driver::{Command, command};
use core::{
    fmt,
    fmt::{Display, Formatter},
};
use deranged::RangedU8;

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
    Offset(u8, u8),
    InvalidLength(u8, RangedU8<0, 128>),
    UnknownCommandError(command::error::Unknown),
    UnexpectedCommandError(command::Error),
}

impl Display for InvalidData {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Offset(expected, received) => write!(
                formatter,
                "received offset of {received} when writing config, but expected offset of {expected}"
            ),
            Self::InvalidLength(received, expected) => write!(
                formatter,
                "received length of {received} when writing config, but expected length of {expected}"
            ),
            Self::UnknownCommandError(_) => {
                formatter.write_str("unable to parse command error payload")
            }
            Self::UnexpectedCommandError(_) => {
                formatter.write_str("received unexpected command error for writing config")
            }
        }
    }
}

impl core::error::Error for InvalidData {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Offset(_, _) => None,
            Self::InvalidLength(_, _) => None,
            Self::UnknownCommandError(unknown) => Some(unknown),
            Self::UnexpectedCommandError(error) => Some(error),
        }
    }
}
