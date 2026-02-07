use crate::driver::HANDSHAKE;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(in crate::driver) enum Data {
    Byte0,
    Byte1,
    Byte2,
    Byte3,
    Byte4,
    Byte5,
    Byte6,
    Byte7,
}

impl Data {
    const BYTE0: u8 = HANDSHAKE[0];
    const BYTE1: u8 = HANDSHAKE[1];
    const BYTE2: u8 = HANDSHAKE[2];
    const BYTE3: u8 = HANDSHAKE[3];
    const BYTE4: u8 = HANDSHAKE[4];
    const BYTE5: u8 = HANDSHAKE[5];
    const BYTE6: u8 = HANDSHAKE[6];
    const BYTE7: u8 = HANDSHAKE[7];

    pub(in crate::driver) fn parse(self, byte: u8) -> Result<Option<Self>, (Error, u16)> {
        match (self, byte) {
            (Self::Byte0, Self::BYTE0) => Ok(Some(Self::Byte1)),
            (Self::Byte1, Self::BYTE1) => Ok(Some(Self::Byte2)),
            (Self::Byte2, Self::BYTE2) => Ok(Some(Self::Byte3)),
            (Self::Byte3, Self::BYTE3) => Ok(Some(Self::Byte4)),
            (Self::Byte4, Self::BYTE4) => Ok(Some(Self::Byte5)),
            (Self::Byte5, Self::BYTE5) => Ok(Some(Self::Byte6)),
            (Self::Byte6, Self::BYTE6) => Ok(Some(Self::Byte7)),
            (Self::Byte7, Self::BYTE7) => Ok(None),
            (Self::Byte0, _) => Err((Error { byte, index: 0 }, 0)),
            (Self::Byte1, _) => Err((Error { byte, index: 1 }, 1)),
            (Self::Byte2, _) => Err((Error { byte, index: 2 }, 2)),
            (Self::Byte3, _) => Err((Error { byte, index: 3 }, 3)),
            (Self::Byte4, _) => Err((Error { byte, index: 4 }, 4)),
            (Self::Byte5, _) => Err((Error { byte, index: 5 }, 5)),
            (Self::Byte6, _) => Err((Error { byte, index: 6 }, 6)),
            (Self::Byte7, _) => Err((Error { byte, index: 7 }, 7)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) struct Error {
    byte: u8,
    index: u8,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "unexpected byte {} at index {}",
            self.byte, self.index
        )
    }
}

impl core::error::Error for Error {}
