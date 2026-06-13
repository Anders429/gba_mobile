use super::{super::Data, Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
    socket,
};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct CloseTcp {
    id: socket::Id,
}

impl CloseTcp {
    pub(in crate::driver::active::flow) fn new(data: &mut Data, id: socket::Id) -> Self {
        data.command = Command::CloseTcpConnection;
        data.data = unsafe { ArrayVec::try_from_iter([id.0]).unwrap_unchecked() };

        Self { id }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::driver::active::flow) enum Response {
    Closed,
    AlreadyClosed,
    AlreadyDisconnected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) enum Error {
    InvalidSocketId {
        expected: socket::Id,
        received: socket::Id,
    },
    Payload(super::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::InvalidSocketId { expected, received } => write!(
                formatter,
                "expected socket ID {expected}, but received socket ID {received}"
            ),
            Self::Payload(_) => formatter.write_str("payload error"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::InvalidSocketId { .. } => None,
            Self::Payload(error) => Some(error),
        }
    }
}

impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        Self::Payload(error)
    }
}

impl Payload for CloseTcp {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::CloseTcpConnection => {
                if data.data.len() == 1 {
                    let received_id =
                        unsafe { data.data.get(0).copied().unwrap_unchecked().into() };
                    if received_id == self.id {
                        Ok(Response::Closed)
                    } else {
                        Err(Error::InvalidSocketId {
                            expected: self.id,
                            received: received_id,
                        })
                    }
                } else {
                    Err(super::Error::InvalidLength {
                        command: Command::CloseTcpConnection,
                        received: data.data.len(),
                        expected: 1,
                    }
                    .into())
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::CloseTcpConnection(
                        command::error::close_tcp_connection::Error::NotConnected,
                    ) => Ok(Response::AlreadyClosed),
                    command::Error::CloseTcpConnection(
                        command::error::close_tcp_connection::Error::NotLoggedIn,
                    ) => Ok(Response::AlreadyDisconnected),
                    _ => Err(super::Error::UnexpectedCommandError(error).into()),
                }
            }
            unexpected => Err(super::Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::CloseTcpConnection, Command::CommandError],
            }
            .into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Payload},
        CloseTcp, Error, Response,
    };
    use crate::{
        arrayvec::ArrayVec,
        driver::{Command, active::packet::payload, command},
        socket,
    };
    use alloc::format;
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use gba_test::test;

    #[test]
    fn new() {
        let id = socket::Id(42);
        let mut data = Data::new();

        let payload = CloseTcp::new(&mut data, id);

        assert_eq!(data.command, Command::CloseTcpConnection);
        assert_eq!(data.data.as_slice(), &[id.0]);
        assert_eq!(payload.id, id);
    }

    #[test]
    fn parse_closed() {
        let id = socket::Id(42);
        let data = Data {
            command: Command::CloseTcpConnection,
            data: assert_ok!(ArrayVec::try_from_iter([id.0])),
        };

        let payload = CloseTcp { id };

        assert_ok_eq!(payload.parse(&data), Response::Closed);
    }

    #[test]
    fn parse_already_closed() {
        let id = socket::Id(42);
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::CloseTcpConnection as u8,
                command::error::close_tcp_connection::Error::NotConnected as u8
            ])),
        };

        let payload = CloseTcp { id };

        assert_ok_eq!(payload.parse(&data), Response::AlreadyClosed);
    }

    #[test]
    fn parse_already_disconnected() {
        let id = socket::Id(42);
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::CloseTcpConnection as u8,
                command::error::close_tcp_connection::Error::NotLoggedIn as u8
            ])),
        };

        let payload = CloseTcp { id };

        assert_ok_eq!(payload.parse(&data), Response::AlreadyDisconnected);
    }

    #[test]
    fn parse_error_invalid_socket_id() {
        let id = socket::Id(42);
        let invalid_id = socket::Id(100);
        let data = Data {
            command: Command::CloseTcpConnection,
            data: assert_ok!(ArrayVec::try_from_iter([invalid_id.0])),
        };

        let payload = CloseTcp { id };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidSocketId {
                expected: id,
                received: invalid_id
            }
        );
    }

    #[test]
    fn parse_error_invalid_length() {
        let id = socket::Id(42);
        let data = Data {
            command: Command::CloseTcpConnection,
            data: ArrayVec::new(),
        };

        let payload = CloseTcp { id };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::InvalidLength {
                command: Command::CloseTcpConnection,
                received: 0,
                expected: 1
            })
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let id = socket::Id(42);
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::CloseUdpConnection as u8,
                command::error::close_udp_connection::Error::NotLoggedIn as u8
            ])),
        };

        let payload = CloseTcp { id };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnexpectedCommandError(
                command::Error::CloseUdpConnection(
                    command::error::close_udp_connection::Error::NotLoggedIn,
                )
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let id = socket::Id(42);
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = CloseTcp { id };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::CloseTcpConnection, Command::CommandError]
            })
        );
    }

    #[test]
    fn error_invalid_socket_id_display() {
        assert_eq!(
            format!(
                "{}",
                Error::InvalidSocketId {
                    expected: socket::Id(42),
                    received: socket::Id(100),
                }
            ),
            "expected socket ID 0x2a, but received socket ID 0x64"
        );
    }
}
