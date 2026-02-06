use crate::engine::Command;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(in crate::engine) enum Step8 {
    MagicByte1,
    MagicByte2,

    HeaderCommand,
    HeaderEmptyByte,
    HeaderLength1,
    HeaderLength2,

    Data { index: u8 },

    Checksum1,
    Checksum2,

    AcknowledgementSignalDevice,
    AcknowledgementSignalCommand,
}

#[derive(Debug)]
pub(in crate::engine) enum Step32 {
    MagicByte,
    HeaderLength,
    Data { index: u8 },
    Checksum,
    AcknowledgementSignal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::engine) enum Error {
    UnsupportedCommand(Command),
    Malformed,
    AdapterInternalError,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnsupportedCommand(command) => {
                write!(formatter, "adapter does not support command {command}")
            }
            Self::Malformed => {
                formatter.write_str("adapter indicated it received a malformed packet")
            }
            Self::AdapterInternalError => {
                formatter.write_str("adapter indicated it encountered an internal error")
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::UnsupportedCommand(_) => None,
            Self::Malformed => None,
            Self::AdapterInternalError => None,
        }
    }
}
