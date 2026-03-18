use core::{
    fmt,
    fmt::{Display, Formatter},
};
use either::Either;

#[derive(Debug)]
pub(in crate::driver) enum Status {
    Idle = 0,
    CallAvailable = 1,
    OutgoingCall = 4,
    IncomingCall = 5,
}

impl TryFrom<u8> for Status {
    type Error = InvalidStatus;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Idle),
            1 => Ok(Self::CallAvailable),
            4 => Ok(Self::OutgoingCall),
            5 => Ok(Self::IncomingCall),
            0xff => Err(InvalidStatus::Disconnected),
            _ => Err(InvalidStatus::Unknown(byte)),
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum InvalidStatus {
    Disconnected,
    Unknown(u8),
}

impl Display for InvalidStatus {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Disconnected => formatter.write_str("phone is disconnected"),
            Self::Unknown(byte) => write!(formatter, "received unknown status byte {byte:#04x}"),
        }
    }
}

impl core::error::Error for InvalidStatus {}

#[derive(Debug)]
pub(in crate::driver) enum Data {
    Status,
    Adapter(Status),
    Metered(Status),
}

impl Data {
    pub(super) fn new() -> Self {
        Self::Status
    }

    pub(super) fn receive_data(self, byte: u8) -> Result<Either<Self, Status>, InvalidStatus> {
        match self {
            Self::Status => byte
                .try_into()
                .map(|status| Either::Left(Self::Adapter(status))),
            // We don't care what the byte is for either of these cases.
            Self::Adapter(status) => Ok(Either::Left(Self::Metered(status))),
            Self::Metered(status) => Ok(Either::Right(status)),
        }
    }
}
