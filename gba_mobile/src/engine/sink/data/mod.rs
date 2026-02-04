pub(in crate::engine) mod begin_session;
pub(in crate::engine) mod command_error;

use crate::engine::command;

use super::{Command, Parsed};
use core::{
    fmt::{self, Display, Formatter},
    num::NonZeroU16,
};
use either::Either;

#[derive(Debug)]
pub(in crate::engine) enum Data {
    BeginSession(begin_session::Data),
    BeginSessionCommandError(command_error::Data),

    EnableSio32CommandError(command_error::Data),
}

impl Data {
    /// On success, returns either the data sink or the parsed result.
    ///
    /// On error, returns error, index of error, expected length, and initial command sink state.
    pub(in crate::engine) fn parse(
        self,
        byte: u8,
    ) -> Result<Either<Self, Parsed>, (Error, u16, NonZeroU16, Command)> {
        match self {
            Self::BeginSession(data) => match data.parse(byte) {
                Ok(Some(data)) => Ok(Either::Left(Self::BeginSession(data))),
                Ok(None) => Ok(Either::Right(Parsed::BeginSession)),
                Err((error, index)) => Err((
                    Error::BeginSession(error),
                    index,
                    unsafe { NonZeroU16::new_unchecked(8) },
                    Command::BeginSession,
                )),
            },
            Self::BeginSessionCommandError(data) => match data.parse(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::BeginSessionCommandError(data))),
                Ok(Either::Right(command_error)) => Ok(Either::Right(
                    Parsed::BeginSessionCommandError(command_error),
                )),
                Err((error, index)) => Err((
                    Error::CommandError(error),
                    index,
                    unsafe { NonZeroU16::new_unchecked(2) },
                    Command::BeginSession,
                )),
            },
            Self::EnableSio32CommandError(data) => match data.parse(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::EnableSio32CommandError(data))),
                Ok(Either::Right(command_error)) => Ok(Either::Right(
                    Parsed::EnableSio32CommandError(command_error),
                )),
                Err((error, index)) => Err((
                    Error::CommandError(error),
                    index,
                    unsafe { NonZeroU16::new_unchecked(2) },
                    Command::BeginSession,
                )),
            },
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::engine) enum Error {
    BeginSession(begin_session::Error),

    CommandError(command::error::Unknown),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::BeginSession(_) => formatter.write_str("begin session handshake error"),

            Self::CommandError(_) => formatter.write_str("command error parsing failure"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::BeginSession(error) => Some(error),

            Self::CommandError(unknown) => Some(unknown),
        }
    }
}
