use super::registration;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub enum Error {
    HeaderM(u8),
    HeaderA(u8),
    Registration(registration::Error),
    Checksum { calculated: u16, received: u16 },
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::HeaderM(byte) => write!(
                formatter,
                "expected first byte of header to be 0x4d, but received {byte:#04x}"
            ),
            Self::HeaderA(byte) => write!(
                formatter,
                "expected second byte of header to be 0x41, but received {byte:#04x}"
            ),
            Self::Registration(_) => formatter.write_str("error reading registration status byte"),
            Self::Checksum {
                calculated,
                received,
            } => write!(
                formatter,
                "calculated checksum of {calculated}, but received {received}"
            ),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::HeaderM(_) => None,
            Self::HeaderA(_) => None,
            Self::Registration(error) => Some(error),
            Self::Checksum { .. } => None,
        }
    }
}

impl From<registration::Error> for Error {
    fn from(error: registration::Error) -> Self {
        Self::Registration(error)
    }
}
