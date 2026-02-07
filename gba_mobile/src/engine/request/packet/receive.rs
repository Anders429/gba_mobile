use crate::engine::{Adapter, Command, adapter, command, sink};
use core::{
    fmt::{self, Display, Formatter},
    num::{NonZeroU8, NonZeroU16},
};

#[derive(Debug)]
pub(in crate::engine) enum Step8 {
    MagicByte1 {
        sink: sink::Command,
        frame: u16,
    },
    MagicByte2 {
        sink: sink::Command,
    },

    HeaderCommand {
        sink: sink::Command,
    },
    HeaderEmptyByte {
        sink: sink::Length,
        command_xor: bool,
    },
    HeaderLength1 {
        sink: sink::Length,
        command_xor: bool,
    },
    HeaderLength2 {
        sink: sink::Length,
        first_byte: u8,
        command_xor: bool,
    },

    Data {
        sink: sink::Data,
        command_xor: bool,
    },

    Checksum1 {
        result: sink::Parsed,
        command_xor: bool,
    },
    Checksum2 {
        result: sink::Parsed,
        first_byte: u8,
        command_xor: bool,
    },

    AcknowledgementSignalDevice {
        result: sink::Parsed,
        command_xor: bool,
    },
    AcknowledgementSignalCommand {
        result: sink::Parsed,
        adapter: Adapter,
        command_xor: bool,
    },
}

#[derive(Debug)]
pub(in crate::engine) enum Step32 {
    MagicByte {
        sink: sink::Command,
        frame: u16,
    },
    HeaderLength {
        sink: sink::Length,
        command_xor: bool,
    },
    Data {
        sink: sink::Data,
        command_xor: bool,
    },
    Checksum {
        result: sink::Parsed,
        command_xor: bool,
    },
    AcknowledgementSignal {
        result: sink::Parsed,
        command_xor: bool,
    },
}

#[derive(Debug)]
pub(in crate::engine) enum Step8Error {
    MagicByte2 {
        sink: sink::Command,
    },

    HeaderCommand {
        sink: sink::Command,
    },
    HeaderEmptyByte {
        sink: sink::Command,
    },
    HeaderLength1 {
        sink: sink::Command,
    },
    HeaderLength2 {
        sink: sink::Command,
        first_byte: u8,
    },

    Data {
        sink: sink::Command,
        index: u16,
        length: NonZeroU16,
    },

    Checksum1 {
        sink: sink::Command,
    },
    Checksum2 {
        sink: sink::Command,
    },

    AcknowledgementSignalDevice {
        sink: sink::Command,
    },
    AcknowledgementSignalCommand {
        sink: sink::Command,
    },
}

#[derive(Debug)]
pub(in crate::engine) enum Step32Error {
    HeaderLength {
        sink: sink::Command,
    },
    Data {
        sink: sink::Command,
        index: u16,
        length: NonZeroU16,
    },
    Checksum {
        sink: sink::Command,
    },
    AcknowledgementSignal {
        sink: sink::Command,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::engine) enum Error {
    MagicValue1(u8),
    MagicValue2(u8),

    UnknownCommand(command::Unknown),
    UnsupportedCommand(sink::command::Error),
    UnexpectedLength(sink::length::Error),

    MalformedData(sink::data::Error),

    Checksum { calculated: u16, received: u16 },
    UnsupportedDevice(adapter::Unknown),
    NonZeroAcknowledgementCommand(NonZeroU8),
}

impl Error {
    pub(in crate::engine) fn command(&self) -> Command {
        match self {
            Self::MagicValue1(_) => Command::MalformedError,
            Self::MagicValue2(_) => Command::MalformedError,

            Self::UnknownCommand(_) => Command::NotSupportedError,
            Self::UnsupportedCommand(_) => Command::NotSupportedError,
            Self::UnexpectedLength(_) => Command::MalformedError,

            Self::MalformedData(_) => Command::MalformedError,

            Self::Checksum { .. } => Command::MalformedError,
            Self::UnsupportedDevice(_) => Command::MalformedError,
            Self::NonZeroAcknowledgementCommand(_) => Command::MalformedError,
        }
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::MagicValue1(byte) => write!(
                formatter,
                "expected first byte of response packet to be 0x99, but received {byte:#04x}"
            ),
            Self::MagicValue2(byte) => write!(
                formatter,
                "expected second byte of response packet to be 0x66, but received {byte:#04x}"
            ),

            Self::UnknownCommand(_) => {
                formatter.write_str("received an invalid command in packet header")
            }
            Self::UnsupportedCommand(_) => formatter.write_str("data sink failed to parse command"),
            Self::UnexpectedLength(_) => formatter.write_str("data sink failed to parse length"),

            Self::MalformedData(_) => formatter.write_str("data sink failed to parse data"),

            Self::Checksum {
                calculated,
                received,
            } => write!(
                formatter,
                "received packet was expected to have checksum of {received}, but was calculated to be {calculated}"
            ),
            Self::UnsupportedDevice(_) => {
                formatter.write_str("unsupported device ID in acknowledgement signal")
            }
            Self::NonZeroAcknowledgementCommand(byte) => write!(
                formatter,
                "received packet acknowledgement signal had command ID of {byte:#04x}, but was expected to be 0x00"
            ),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::MagicValue1(_) => None,
            Self::MagicValue2(_) => None,

            Self::UnknownCommand(unknown) => Some(unknown),
            Self::UnsupportedCommand(error) => Some(error),
            Self::UnexpectedLength(error) => Some(error),

            Self::MalformedData(error) => Some(error),

            Self::Checksum { .. } => None,
            Self::UnsupportedDevice(unknown) => Some(unknown),
            Self::NonZeroAcknowledgementCommand(_) => None,
        }
    }
}
