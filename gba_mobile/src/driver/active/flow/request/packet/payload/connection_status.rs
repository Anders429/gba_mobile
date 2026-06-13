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

#[derive(Debug, Eq, PartialEq)]
pub(in crate::driver::active::flow) enum Response {
    Connected,
    NotConnected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
#[repr(u8)]
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

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Payload},
        ConnectionStatus, Error, Response, Status,
    };
    use crate::{
        ArrayVec,
        driver::{Command, active::flow::packet::payload, command},
    };
    use alloc::format;
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use core::marker::PhantomData;
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();

        ConnectionStatus::new(&mut data);

        assert_eq!(
            data,
            Data {
                command: Command::TelephoneStatus,
                data: ArrayVec::new(),
            }
        );
    }

    #[test]
    fn parse_connected() {
        let data = Data {
            command: Command::TelephoneStatus,
            data: assert_ok!(ArrayVec::try_from_iter([
                Status::OutgoingCall as u8,
                0x4d, // Adapter byte. This one is for the blue adapter.
                0x00, // Rate limit byte. This one is rate limited.
            ])),
        };

        let payload = ConnectionStatus {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::Connected);
    }

    #[test]
    fn parse_not_connected() {
        let data = Data {
            command: Command::TelephoneStatus,
            data: assert_ok!(ArrayVec::try_from_iter([
                Status::Idle as u8,
                0x48, // Adapter byte. This one is for the yellow or red adapters.
                0xf0, // Rate limit byte. This is one is not rate limited.
            ])),
        };

        let payload = ConnectionStatus {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::NotConnected);
    }

    #[test]
    fn parse_error_disconnected() {
        let data = Data {
            command: Command::TelephoneStatus,
            data: assert_ok!(ArrayVec::try_from_iter([
                0xff, // This byte indicates the phone is disconnected.
                0x4d, // Adapter byte. This one is for the blue adapter.
                0x00, // Rate limit byte. This one is rate limited.
            ])),
        };

        let payload = ConnectionStatus {
            _private: PhantomData,
        };

        assert_err_eq!(payload.parse(&data), Error::Disconnected);
    }

    #[test]
    fn parse_error_unknown_status() {
        let data = Data {
            command: Command::TelephoneStatus,
            data: assert_ok!(ArrayVec::try_from_iter([
                0xaa, // This is an invalid status byte that we don't expect the adapter to return.
                0x4d, // Adapter byte. This one is for the blue adapter.
                0x00, // Rate limit byte. This one is rate limited.
            ])),
        };

        let payload = ConnectionStatus {
            _private: PhantomData,
        };

        assert_err_eq!(payload.parse(&data), Error::UnknownStatus(0xaa));
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::TelephoneStatus,
            data: ArrayVec::new(),
        };

        let payload = ConnectionStatus {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::InvalidLength {
                command: Command::TelephoneStatus,
                received: 0,
                expected: 3
            })
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::BeginSession as u8,
                command::error::begin_session::Error::AlreadyActive as u8,
            ])),
        };

        let payload = ConnectionStatus {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnexpectedCommandError(
                command::Error::BeginSession(command::error::begin_session::Error::AlreadyActive)
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = ConnectionStatus {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::TelephoneStatus, Command::CommandError]
            })
        );
    }

    #[test]
    fn error_display_disconnected() {
        assert_eq!(format!("{}", Error::Disconnected), "link is disconnected",);
    }

    #[test]
    fn error_display_unknown_status() {
        assert_eq!(
            format!("{}", Error::UnknownStatus(0x42)),
            "received unknown status byte 0x42",
        );
    }

    #[test]
    fn error_display_payload() {
        assert_eq!(
            format!(
                "{}",
                Error::Payload(payload::Error::InvalidLength {
                    command: Command::DialTelephone,
                    received: 0,
                    expected: 3
                })
            ),
            "payload error",
        )
    }
}
