use crate::driver::Command;

use super::{Payload, ReceiveCommand, ReceiveData, ReceiveLength};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

#[derive(Debug)]
pub(in crate::driver) enum Error<Payload>
where
    Payload: self::Payload,
{
    ReceiveCommand(<Payload::ReceiveCommand as ReceiveCommand>::Error),
    ReceiveLength(<Payload::ReceiveLength as ReceiveLength>::Error),
    ReceiveData(<Payload::ReceiveData as ReceiveData>::Error),
}

impl<Payload> Error<Payload>
where
    Payload: self::Payload,
{
    pub(in super::super) fn command(&self) -> Command {
        match self {
            Self::ReceiveCommand(_) => Command::NotSupportedError,
            Self::ReceiveLength(_) => Command::MalformedError,
            Self::ReceiveData(_) => Command::MalformedError,
        }
    }
}

impl<Payload> Clone for Error<Payload>
where
    Payload: self::Payload,
{
    fn clone(&self) -> Self {
        match self {
            Self::ReceiveCommand(error) => Self::ReceiveCommand(error.clone()),
            Self::ReceiveLength(error) => Self::ReceiveLength(error.clone()),
            Self::ReceiveData(error) => Self::ReceiveData(error.clone()),
        }
    }
}

impl<Payload> Display for Error<Payload>
where
    Payload: self::Payload,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::ReceiveCommand(_) => formatter.write_str("failed to parse received command"),
            Self::ReceiveLength(_) => formatter.write_str("failed to parse received length"),
            Self::ReceiveData(_) => formatter.write_str("failed to parse received data"),
        }
    }
}

impl<Payload> core::error::Error for Error<Payload>
where
    Payload: self::Payload,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ReceiveCommand(error) => Some(error),
            Self::ReceiveLength(error) => Some(error),
            Self::ReceiveData(error) => Some(error),
        }
    }
}
