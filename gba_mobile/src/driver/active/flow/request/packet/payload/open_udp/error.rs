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
            Command::OpenUdpConnection,
            Command::CommandError
        )
    }
}

impl core::error::Error for UnsupportedCommand {}

#[derive(Clone, Debug)]
pub(in crate::driver) enum InvalidLength {
    OpenUdpConnection(u8),
    CommandError(u8),
}

impl Display for InvalidLength {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::OpenUdpConnection(length) => write!(
                formatter,
                "received length of {length} for {} packet, but expected length of 1",
                Command::OpenUdpConnection
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
    UnknownCommandError(command::error::Unknown),
    UnexpectedCommandError(command::Error),
}

impl Display for InvalidData {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnknownCommandError(_) => {
                formatter.write_str("unable to parse command error payload")
            }
            Self::UnexpectedCommandError(_) => formatter
                .write_str("received unexpected command error while opening UDP connection"),
        }
    }
}

impl core::error::Error for InvalidData {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::UnknownCommandError(unknown) => Some(unknown),
            Self::UnexpectedCommandError(error) => Some(error),
        }
    }
}
