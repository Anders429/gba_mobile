use super::{Command, Data, Parsed, data};
use core::{
    fmt::{self, Display, Formatter},
    num::NonZeroU16,
};
use either::Either;

#[derive(Debug)]
pub(in crate::driver) enum Length {
    BeginSession,
    BeginSessionCommandError,

    EnableSio32(bool),
    EnableSio32CommandError,

    WaitForCall,
    WaitForCallCommandError,

    EndSession,
    EndSessionCommandError,
}

impl Length {
    pub(in crate::driver) fn parse(
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
            Self::EnableSio32(enabled) => {
                if let Some(nonzero_length) = NonZeroU16::new(length) {
                    Err((Error::EnableSio32(nonzero_length), Command::EnableSio32))
                } else {
                    Ok(Either::Right(Parsed::EnableSio32(enabled)))
                }
            }
            Self::EnableSio32CommandError => {
                if length == 2 {
                    Ok(Either::Left(Data::EnableSio32CommandError(
                        data::command_error::Data::Command,
                    )))
                } else {
                    Err((Error::CommandError(length), Command::EnableSio32))
                }
            }

            Self::WaitForCall => {
                if let Some(nonzero_length) = NonZeroU16::new(length) {
                    Err((Error::WaitForCall(nonzero_length), Command::WaitForCall))
                } else {
                    Ok(Either::Right(Parsed::WaitForCall))
                }
            }
            Self::WaitForCallCommandError => {
                if length == 2 {
                    Ok(Either::Left(Data::WaitForCallCommandError(
                        data::command_error::Data::Command,
                    )))
                } else {
                    Err((Error::CommandError(length), Command::WaitForCall))
                }
            }

            Self::EndSession => {
                if let Some(nonzero_length) = NonZeroU16::new(length) {
                    Err((Error::EndSession(nonzero_length), Command::EndSession))
                } else {
                    Ok(Either::Right(Parsed::EndSession))
                }
            }
            Self::EndSessionCommandError => {
                if length == 2 {
                    Ok(Either::Left(Data::EndSessionCommandError(
                        data::command_error::Data::Command,
                    )))
                } else {
                    Err((Error::CommandError(length), Command::EndSession))
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) enum Error {
    BeginSession(u16),
    EnableSio32(NonZeroU16),

    WaitForCall(NonZeroU16),

    EndSession(NonZeroU16),

    CommandError(u16),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::BeginSession(length) => write!(formatter, "received {length}; expected 8"),
            Self::EnableSio32(nonzero_length) => {
                write!(formatter, "received {nonzero_length}; expected 0")
            }
            Self::WaitForCall(nonzero_length) => {
                write!(formatter, "received {nonzero_length}; expected 0")
            }
            Self::EndSession(nonzero_length) => {
                write!(formatter, "received {nonzero_length}; expected 0")
            }
            Self::CommandError(length) => write!(formatter, "received {length}; expected 2"),
        }
    }
}

impl core::error::Error for Error {}
