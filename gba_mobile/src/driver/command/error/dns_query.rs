use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::driver) enum Error {
    NotLoggedIn = 0x01,
    LookupFailed = 0x02,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::NotLoggedIn => formatter.write_str("not logged in"),
            Self::LookupFailed => formatter.write_str("lookup failed"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x01 => Ok(Self::NotLoggedIn),
            0x02 => Ok(Self::LookupFailed),
            _ => Err(UnknownError(byte)),
        }
    }
}
