use super::Length;
use crate::engine;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(in crate::engine) enum Command {
    BeginSession,
}

impl Command {
    pub(in crate::engine) fn parse(
        self,
        command: engine::Command,
    ) -> Result<Length, (Error, Command)> {
        match self {
            Self::BeginSession => match command {
                engine::Command::BeginSession => Ok(Length::BeginSession),
                engine::Command::CommandError => Ok(Length::BeginSessionCommandError),
                _ => Err((Error::BeginSession(command), self)),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::engine) enum Error {
    BeginSession(engine::Command),
}

impl Error {
    fn fmt_error(
        formatter: &mut Formatter,
        command: engine::Command,
        expected: &[engine::Command],
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
                &[engine::Command::BeginSession, engine::Command::CommandError],
            ),
        }
    }
}

impl core::error::Error for Error {}
