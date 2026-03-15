use super::{Payload, payload};
use crate::driver::{Command, adapter, command};
use core::{
    fmt::{self, Display, Formatter},
    num::NonZeroU8,
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Send {
    UnsupportedCommand(Command),
    Malformed,
    AdapterInternalError,
}

impl Display for Send {
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

impl core::error::Error for Send {}

#[derive(Debug)]
pub(in crate::driver) enum Receive<Payload>
where
    Payload: self::Payload,
{
    MagicValue1(u8),
    MagicValue2(u8),

    UnknownCommand(command::Unknown),
    LengthTooLarge(u16),

    Payload(payload::Error<Payload>),

    Checksum { calculated: u16, received: u16 },
    UnsupportedDevice(adapter::Unknown),
    NonZeroFooterCommand(NonZeroU8),
}

impl<Payload> Receive<Payload>
where
    Payload: self::Payload,
{
    pub(super) fn command(&self) -> Command {
        match self {
            Self::MagicValue1(_) => Command::MalformedError,
            Self::MagicValue2(_) => Command::MalformedError,

            Self::UnknownCommand(_) => Command::NotSupportedError,
            Self::LengthTooLarge(_) => Command::MalformedError,

            Self::Payload(error) => error.command(),

            Self::Checksum { .. } => Command::MalformedError,
            Self::UnsupportedDevice(_) => Command::MalformedError,
            Self::NonZeroFooterCommand(_) => Command::MalformedError,
        }
    }
}

impl<Payload> Clone for Receive<Payload>
where
    Payload: self::Payload,
{
    fn clone(&self) -> Self {
        match self {
            Self::MagicValue1(byte) => Self::MagicValue1(*byte),
            Self::MagicValue2(byte) => Self::MagicValue2(*byte),

            Self::UnknownCommand(unknown) => Self::UnknownCommand(unknown.clone()),
            Self::LengthTooLarge(length) => Self::LengthTooLarge(*length),

            Self::Payload(error) => Self::Payload(error.clone()),

            Self::Checksum {
                calculated,
                received,
            } => Self::Checksum {
                calculated: *calculated,
                received: *received,
            },
            Self::UnsupportedDevice(unknown) => Self::UnsupportedDevice(unknown.clone()),
            Self::NonZeroFooterCommand(byte) => Self::NonZeroFooterCommand(*byte),
        }
    }
}

impl<Payload> Display for Receive<Payload>
where
    Payload: self::Payload,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::MagicValue1(byte) => write!(
                formatter,
                "expected first byte of 0x99, but received {byte:#04x}"
            ),
            Self::MagicValue2(byte) => write!(
                formatter,
                "expected first byte of 0x99, but received {byte:#04x}"
            ),

            Self::UnknownCommand(_) => {
                formatter.write_str("received an invalid command in packet header")
            }
            Self::LengthTooLarge(length) => write!(
                formatter,
                "received response packet length of {length}, but maximum supported length is 255"
            ),

            Self::Payload(_) => formatter.write_str("error while parsing packet's payload"),

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
            Self::NonZeroFooterCommand(byte) => write!(
                formatter,
                "received packet's footer had command ID of {byte:#04x}, but was expected to be 0x00"
            ),
        }
    }
}

impl<Payload> core::error::Error for Receive<Payload>
where
    Payload: self::Payload,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::MagicValue1(_) => None,
            Self::MagicValue2(_) => None,

            Self::UnknownCommand(unknown) => Some(unknown),
            Self::LengthTooLarge(_) => None,

            Self::Payload(error) => Some(error),

            Self::Checksum { .. } => None,
            Self::UnsupportedDevice(unknown) => Some(unknown),
            Self::NonZeroFooterCommand(_) => None,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum Error<Payload>
where
    Payload: self::Payload,
{
    Send(Send),
    Receive(Receive<Payload>),
}

impl<Payload> Clone for Error<Payload>
where
    Payload: self::Payload,
{
    fn clone(&self) -> Self {
        match self {
            Self::Send(error) => Self::Send(error.clone()),
            Self::Receive(error) => Self::Receive(error.clone()),
        }
    }
}

impl<Payload> Display for Error<Payload>
where
    Payload: self::Payload,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Send(_) => formatter.write_str("error while sending packet"),
            Self::Receive(_) => formatter.write_str("error while receiving packet"),
        }
    }
}

impl<Payload> core::error::Error for Error<Payload>
where
    Payload: self::Payload,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Send(error) => Some(error),
            Self::Receive(error) => Some(error),
        }
    }
}

impl<Payload> From<Send> for Error<Payload>
where
    Payload: self::Payload,
{
    fn from(error: Send) -> Self {
        Self::Send(error)
    }
}

impl<Payload> From<Receive<Payload>> for Error<Payload>
where
    Payload: self::Payload,
{
    fn from(error: Receive<Payload>) -> Self {
        Self::Receive(error)
    }
}
