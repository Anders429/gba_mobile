use super::{super::Data, Payload, command_error};
use crate::{ArrayVec, driver::Command};
use core::{
    fmt,
    fmt::{Display, Formatter},
    marker::PhantomData,
};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct ConnectionStatus {
    _private: PhantomData<()>,
}

impl ConnectionStatus {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::TelephoneStatus;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum Response {
    Connected,
    NotConnected,
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Disconnected,
    UnknownStatus(u8),
    Payload(super::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Disconnected => formatter.write_str("link is disconnected"),
            Self::UnknownStatus(byte) => {
                write!(formatter, "received unknown status byte {byte:#04x}")
            }
            Self::Payload(_) => formatter.write_str("payload error"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Disconnected => None,
            Self::UnknownStatus(_) => None,
            Self::Payload(error) => Some(error),
        }
    }
}

impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        Self::Payload(error)
    }
}

#[derive(Debug)]
enum Status {
    Idle = 0,
    CallAvailable = 1,
    OutgoingCall = 4,
    IncomingCall = 5,
}

impl TryFrom<u8> for Status {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Idle),
            1 => Ok(Self::CallAvailable),
            4 => Ok(Self::OutgoingCall),
            5 => Ok(Self::IncomingCall),
            0xff => Err(Error::Disconnected),
            _ => Err(Error::UnknownStatus(byte)),
        }
    }
}

impl Payload for ConnectionStatus {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::TelephoneStatus => {
                if data.data.len() == 3 {
                    unsafe { data.data.get(0).copied().unwrap_unchecked() }
                        .try_into()
                        .map(|status| match status {
                            Status::Idle | Status::CallAvailable => Response::NotConnected,
                            Status::IncomingCall | Status::OutgoingCall => Response::Connected,
                        })
                } else {
                    Err(super::Error::InvalidLength {
                        command: Command::TelephoneStatus,
                        received: data.data.len(),
                        expected: 3,
                    }
                    .into())
                }
            }
            Command::CommandError => {
                Err(super::Error::UnexpectedCommandError(command_error::parse(&data.data)?).into())
            }
            unexpected => Err(super::Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::TelephoneStatus, Command::CommandError],
            }
            .into()),
        }
    }
}
