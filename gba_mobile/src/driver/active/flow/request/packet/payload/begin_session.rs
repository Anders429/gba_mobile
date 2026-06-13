use super::{super::Data, Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::{
    fmt,
    fmt::{Display, Formatter},
    marker::PhantomData,
};

const HANDSHAKE: [u8; 8] = [0x4e, 0x49, 0x4e, 0x54, 0x45, 0x4e, 0x44, 0x4f];

#[derive(Debug)]
pub(in crate::driver::active::flow) struct BeginSession {
    _private: PhantomData<()>,
}

impl BeginSession {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::BeginSession;
        data.data = unsafe { ArrayVec::try_from_iter(HANDSHAKE).unwrap_unchecked() };

        Self {
            _private: PhantomData,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::driver::active::flow) enum Response {
    BeginSession,
    AlreadyActive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) enum Error {
    Handshake { byte: u8, index: usize },
    Payload(super::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Handshake { byte, index } => write!(
                formatter,
                "unexpected byte {byte:#04x} at index {index}; expected {:#04x}",
                HANDSHAKE[*index],
            ),
            Self::Payload(_) => formatter.write_str("payload error"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Handshake { .. } => None,
            Self::Payload(error) => Some(error),
        }
    }
}

impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        Self::Payload(error)
    }
}

impl Payload for BeginSession {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::BeginSession => {
                if data.data.len() == 8 {
                    for (index, (received, expected)) in data
                        .data
                        .iter()
                        .copied()
                        .zip(HANDSHAKE.into_iter())
                        .enumerate()
                    {
                        if received != expected {
                            return Err(Error::Handshake {
                                byte: received,
                                index,
                            });
                        }
                    }
                    Ok(Response::BeginSession)
                } else {
                    Err(super::Error::InvalidLength {
                        command: Command::BeginSession,
                        received: data.data.len(),
                        expected: 8,
                    }
                    .into())
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::BeginSession(
                        command::error::begin_session::Error::AlreadyActive,
                    ) => Ok(Response::AlreadyActive),
                    _ => Err(super::Error::UnexpectedCommandError(error).into()),
                }
            }
            unexpected => Err(super::Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::BeginSession, Command::CommandError],
            }
            .into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Payload},
        BeginSession, Error, HANDSHAKE, Response,
    };
    use crate::{
        ArrayVec,
        driver::{Command, active::packet::payload, command},
    };
    use alloc::format;
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use core::marker::PhantomData;
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();

        BeginSession::new(&mut data);

        assert_eq!(data.command, Command::BeginSession);
        assert_eq!(data.data.as_slice(), HANDSHAKE);
    }

    #[test]
    fn parse_begin_session() {
        let data = Data {
            command: Command::BeginSession,
            data: assert_ok!(ArrayVec::try_from_iter(HANDSHAKE)),
        };

        let payload = BeginSession {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::BeginSession);
    }

    #[test]
    fn parse_already_active() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::BeginSession as u8,
                command::error::begin_session::Error::AlreadyActive as u8
            ])),
        };

        let payload = BeginSession {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::AlreadyActive);
    }

    #[test]
    fn parse_error_handshake() {
        let data = Data {
            command: Command::BeginSession,
            data: assert_ok!(ArrayVec::try_from_iter(*b"SEGA    ")),
        };

        let payload = BeginSession {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Handshake {
                byte: b'S',
                index: 0,
            }
        );
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::BeginSession,
            data: ArrayVec::new(),
        };

        let payload = BeginSession {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::InvalidLength {
                command: Command::BeginSession,
                received: 0,
                expected: 8
            })
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::BeginSession as u8,
                command::error::begin_session::Error::InvalidContents as u8
            ])),
        };

        let payload = BeginSession {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnexpectedCommandError(
                command::Error::BeginSession(command::error::begin_session::Error::InvalidContents,)
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::InternalError,
            data: ArrayVec::new(),
        };

        let payload = BeginSession {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnsupportedCommand {
                received: Command::InternalError,
                expected: &[Command::BeginSession, Command::CommandError]
            })
        );
    }

    #[test]
    fn error_handshake_display() {
        assert_eq!(
            format!(
                "{}",
                Error::Handshake {
                    byte: b'A',
                    index: 6
                }
            ),
            "unexpected byte 0x41 at index 6; expected 0x44"
        );
    }
}
