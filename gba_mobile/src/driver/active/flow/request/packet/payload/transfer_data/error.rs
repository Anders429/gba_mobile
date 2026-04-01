use crate::{
    driver::{Command, command},
    socket,
};
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
            "unsupported command {}; supported commands are {}, {}, and {}",
            self.0,
            Command::TransferData,
            Command::ConnectionClosed,
            Command::CommandError
        )
    }
}

impl core::error::Error for UnsupportedCommand {}

#[derive(Clone, Debug)]
pub(in crate::driver) enum InvalidLength {
    TransferData,
    ConnectionClosed,
    CommandError(u8),
}

impl Display for InvalidLength {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::TransferData => write!(
                formatter,
                "received length of 0 for {} packet, but expected nonzero length",
                Command::TransferData,
            ),
            Self::ConnectionClosed => write!(
                formatter,
                "received length of 0 for {} packet, but expected nonzero length",
                Command::ConnectionClosed,
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
    IncorrectSocketId {
        received: socket::Id,
        expected: socket::Id,
    },
    UnknownCommandError(command::error::Unknown),
    UnexpectedCommandError(command::Error),
}

impl Display for InvalidData {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::IncorrectSocketId { received, expected } => write!(
                formatter,
                "received socket ID {received}, but expected socket ID {expected}"
            ),
            Self::UnknownCommandError(_) => {
                formatter.write_str("unable to parse command error payload")
            }
            Self::UnexpectedCommandError(_) => {
                formatter.write_str("received unexpected command error for transfer data")
            }
        }
    }
}

impl core::error::Error for InvalidData {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::IncorrectSocketId { .. } => None,
            Self::UnknownCommandError(unknown) => Some(unknown),
            Self::UnexpectedCommandError(error) => Some(error),
        }
    }
}
