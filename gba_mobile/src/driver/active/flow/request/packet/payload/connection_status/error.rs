use super::data;
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
            Command::TelephoneStatus,
            Command::CommandError
        )
    }
}

impl core::error::Error for UnsupportedCommand {}

#[derive(Clone, Debug)]
pub(in crate::driver) enum InvalidLength {
    TelephoneStatus(u8),
    CommandError(u8),
}

impl Display for InvalidLength {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::TelephoneStatus(length) => write!(
                formatter,
                "received length of {length} for {} packet, but expected length of 3",
                Command::TelephoneStatus,
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
    TelephoneStatus(data::InvalidStatus),
    UnknownCommandError(command::error::Unknown),
    UnexpectedCommandError(command::Error),
}

impl Display for InvalidData {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::TelephoneStatus(_) => formatter.write_str("connection status data error"),
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
            Self::TelephoneStatus(error) => Some(error),
            Self::UnknownCommandError(unknown) => Some(unknown),
            Self::UnexpectedCommandError(error) => Some(error),
        }
    }
}
