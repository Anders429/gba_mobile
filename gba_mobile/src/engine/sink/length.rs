use super::{Command, Data, Parsed, data};
use core::{
    fmt::{self, Display, Formatter},
    num::NonZeroU16,
};
use either::Either;

#[derive(Debug)]
pub(in crate::engine) enum Length {
    BeginSession,
    BeginSessionCommandError,

    EnableSio32(bool),
    EnableSio32CommandError,
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
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::engine) enum Error {
    BeginSession(u16),
    EnableSio32(NonZeroU16),

    CommandError(u16),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::BeginSession(length) => write!(formatter, "received {length}; expected 8"),
            Self::EnableSio32(nonzero_length) => {
                write!(formatter, "received {nonzero_length}; expected 0")
            }
            Self::CommandError(length) => write!(formatter, "received {length}; expected 2"),
        }
    }
}

impl core::error::Error for Error {}
