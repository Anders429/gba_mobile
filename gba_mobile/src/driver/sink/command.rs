use super::Length;
use crate::driver;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(in crate::driver) enum Command {
    BeginSession,
    EnableSio32,
}

impl Command {
    pub(in crate::driver) fn parse(
        self,
        command: driver::Command,
    ) -> Result<Length, (Error, Command)> {
        match self {
            Self::BeginSession => match command {
                driver::Command::BeginSession => Ok(Length::BeginSession),
                driver::Command::CommandError => Ok(Length::BeginSessionCommandError),
                _ => Err((Error::BeginSession(command), self)),
            },
            Self::EnableSio32 => match command {
                driver::Command::Sio32Mode => Ok(Length::EnableSio32(true)),
                driver::Command::Reset => Ok(Length::EnableSio32(false)),
                driver::Command::CommandError => Ok(Length::EnableSio32CommandError),
                _ => Err((Error::EnableSio32(command), self)),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) enum Error {
    BeginSession(driver::Command),
    EnableSio32(driver::Command),
}

impl Error {
    fn fmt_error(
        formatter: &mut Formatter,
        command: driver::Command,
        expected: &[driver::Command],
    ) -> fmt::Result {
        write!(
            formatter,
            "received command {command}, but expected one of ["
        )?;
        let mut first = true;
        for expected_command in expected {
            if !first {
                formatter.write_str(", ")?;
            }
            write!(formatter, "{expected_command}")?;
            first = false;
        }
        formatter.write_str("]")
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::BeginSession(command) => Self::fmt_error(
                formatter,
                *command,
                &[driver::Command::BeginSession, driver::Command::CommandError],
            ),
            Self::EnableSio32(command) => Self::fmt_error(
                formatter,
                *command,
                &[
                    driver::Command::Sio32Mode,
                    driver::Command::Reset,
                    driver::Command::CommandError,
                ],
            ),
        }
    }
}

impl core::error::Error for Error {}
