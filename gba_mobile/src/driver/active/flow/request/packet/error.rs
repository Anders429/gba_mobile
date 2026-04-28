use super::Payload;
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

#[derive(Clone, Debug)]
pub(in crate::driver) enum Receive {
    MagicValue2(u8),

    UnknownCommand(command::Unknown),
    LengthTooLarge(u16),

    Checksum { calculated: u16, received: u16 },
    UnsupportedDevice(adapter::Unknown),
    NonZeroFooterCommand(NonZeroU8),
}

impl Receive {
    pub(super) fn command(&self) -> Command {
        match self {
            Self::MagicValue2(_) => Command::MalformedError,

            Self::UnknownCommand(_) => Command::NotSupportedError,
            Self::LengthTooLarge(_) => Command::MalformedError,

            Self::Checksum { .. } => Command::MalformedError,
            Self::UnsupportedDevice(_) => Command::MalformedError,
            Self::NonZeroFooterCommand(_) => Command::MalformedError,
        }
    }
}

impl Display for Receive {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::MagicValue2(byte) => write!(
                formatter,
                "expected second byte of 0x66, but received {byte:#04x}"
            ),

            Self::UnknownCommand(_) => {
                formatter.write_str("received an invalid command in packet header")
            }
            Self::LengthTooLarge(length) => write!(
                formatter,
                "received response packet length of {length}, but maximum supported length is 255"
            ),

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

impl core::error::Error for Receive {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::MagicValue2(_) => None,

            Self::UnknownCommand(unknown) => Some(unknown),
            Self::LengthTooLarge(_) => None,

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
    Receive(Receive),
    Payload(Payload::Error),
}

impl<Payload> Clone for Error<Payload>
where
    Payload: self::Payload,
{
    fn clone(&self) -> Self {
        match self {
            Self::Send(error) => Self::Send(error.clone()),
            Self::Receive(error) => Self::Receive(error.clone()),
            Self::Payload(error) => Self::Payload(error.clone()),
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
            Self::Payload(_) => formatter.write_str("error interpreting payload"),
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
            Self::Payload(error) => Some(error),
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

impl<Payload> From<Receive> for Error<Payload>
where
    Payload: self::Payload,
{
    fn from(error: Receive) -> Self {
        Self::Receive(error)
    }
}
