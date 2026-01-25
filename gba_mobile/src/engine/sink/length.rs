use super::{Command, Data, Parsed, data};
use core::{
    fmt,
    fmt::{Display, Formatter},
};
use either::Either;

#[derive(Debug)]
pub(in crate::engine) enum Length {
    BeginSession,
    BeginSessionCommandError,
}

impl Length {
    pub(in crate::engine) fn parse(
        self,
        length: u16,
    ) -> Result<Either<Data, Parsed>, (Error, Command)> {
        match self {
            Self::BeginSession => {
                if length == 8 {
                    Ok(Either::Left(Data::BeginSession(
                        data::begin_session::Data::Byte0,
                    )))
                } else {
                    Err((Error::BeginSession(length), Command::BeginSession))
                }
            }
            Self::BeginSessionCommandError => {
                if length == 2 {
                    Ok(Either::Left(Data::BeginSessionCommandError(
                        data::command_error::Data::Command,
                    )))
                } else {
                    Err((Error::CommandError(length), Command::BeginSession))
                }
            }
        }
    }
}

#[derive(Debug)]
pub(in crate::engine) enum Error {
    BeginSession(u16),

    CommandError(u16),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::BeginSession(length) => write!(formatter, "received {length}; expected 8"),

            Self::CommandError(length) => write!(formatter, "received {length}; expected 2"),
        }
    }
}

impl core::error::Error for Error {}
