use crate::driver::{Command, command};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    UnsupportedCommand {
        received: Command,
        expected: &'static [Command],
    },
    InvalidLength {
        command: Command,
        received: u8,
        expected: u8,
    },
    UnknownCommandError(command::error::Unknown),
    UnexpectedCommandError(command::Error),
}

fn fmt_unsupported_command(
    formatter: &mut Formatter,
    received: Command,
    expected: &'static [Command],
) -> fmt::Result {
    write!(formatter, "unsupported command {received}")?;
    if let Some((last, list)) = expected.split_last() {
        if list.len() <= 1 {
            if let Some(first) = list.first() {
                write!(formatter, "; supported commands are {first} and {last}")?;
            } else {
                write!(formatter, "; expected {last}")?;
            }
        } else {
            formatter.write_str("; supported commands are ")?;
            for command in list {
                write!(formatter, "{command}, ")?;
            }
            write!(formatter, "and {last}")?;
        }
    }

    Ok(())
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnsupportedCommand { received, expected } => {
                fmt_unsupported_command(formatter, *received, expected)
            }
            Self::InvalidLength {
                command,
                received,
                expected,
            } => write!(
                formatter,
                "received length of {received} for {command} packet, but expected length of {expected}"
            ),
            Self::UnknownCommandError(_) => {
                formatter.write_str("unable to parse command error payload")
            }
            Self::UnexpectedCommandError(_) => {
                formatter.write_str("received unexpected command error")
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::UnsupportedCommand { .. } => None,
            Self::InvalidLength { .. } => None,
            Self::UnknownCommandError(unknown) => Some(unknown),
            Self::UnexpectedCommandError(error) => Some(error),
        }
    }
}
