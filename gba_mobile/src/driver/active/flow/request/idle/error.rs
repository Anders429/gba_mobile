use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Sio8(u8),
    Sio32(u32),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Sio8(byte) => write!(
                formatter,
                "adapter did not respond with an idle byte while no packet was being processed; received {byte:#04x}, expected 0xd2"
            ),
            Self::Sio32(byte) => write!(
                formatter,
                "adapter did not respond with idle bytes while no packet was being processed; received {byte:#010x}; expected 0xd2d2d2d2"
            ),
        }
    }
}

impl core::error::Error for Error {}
