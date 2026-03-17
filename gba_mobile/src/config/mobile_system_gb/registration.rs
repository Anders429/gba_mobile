use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Registration {
    Incomplete = 0x01,
    Complete = 0x81,
}

impl Registration {
    pub(super) fn try_from(byte: u8) -> Result<Self, Error> {
        match byte {
            0x01 => Ok(Self::Incomplete),
            0x81 => Ok(Self::Complete),
            _ => Err(Error(byte)),
        }
    }
}

#[derive(Debug)]
pub struct Error(u8);

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "expected either 0x01 or 0x81, but received {:#04x}",
            self.0
        )
    }
}

impl core::error::Error for Error {}
