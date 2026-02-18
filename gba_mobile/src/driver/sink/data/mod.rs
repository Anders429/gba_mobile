pub(in crate::driver) mod begin_session;
pub(in crate::driver) mod command_error;

use crate::driver::command;

use super::{Command, Parsed};
use core::{
    fmt::{self, Display, Formatter},
    num::NonZeroU16,
};
use either::Either;

#[derive(Debug)]
pub(in crate::driver) enum Data {
    BeginSession(begin_session::Data),
    BeginSessionCommandError(command_error::Data),

    EnableSio32CommandError(command_error::Data),

    WaitForCallCommandError(command_error::Data),
    CallCommandError(command_error::Data),

    EndSessionCommandError(command_error::Data),
}

impl Data {
    /// On success, returns either the data sink or the parsed result.
    ///
    /// On error, returns error, index of error, expected length, and initial command sink state.
    pub(in crate::driver) fn parse(
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
                    Command::EnableSio32,
                )),
            },
            Self::WaitForCallCommandError(data) => match data.parse(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::WaitForCallCommandError(data))),
                Ok(Either::Right(command_error)) => Ok(Either::Right(
                    Parsed::WaitForCallCommandError(command_error),
                )),
                Err((error, index)) => Err((
                    Error::CommandError(error),
                    index,
                    unsafe { NonZeroU16::new_unchecked(2) },
                    Command::WaitForCall,
                )),
            },
            Self::CallCommandError(data) => match data.parse(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::CallCommandError(data))),
                Ok(Either::Right(command_error)) => {
                    Ok(Either::Right(Parsed::CallCommandError(command_error)))
                }
                Err((error, index)) => Err((
                    Error::CommandError(error),
                    index,
                    unsafe { NonZeroU16::new_unchecked(2) },
                    Command::Call,
                )),
            },
            Self::EndSessionCommandError(data) => match data.parse(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::EndSessionCommandError(data))),
                Ok(Either::Right(command_error)) => {
                    Ok(Either::Right(Parsed::EndSessionCommandError(command_error)))
                }
                Err((error, index)) => Err((
                    Error::CommandError(error),
                    index,
                    unsafe { NonZeroU16::new_unchecked(2) },
                    Command::EndSession,
                )),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) enum Error {
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
