use super::{super::Data, Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
    socket,
};
use core::{
    fmt::{self, Display, Formatter},
    iter,
};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct TransferData {
    id: socket::Id,
}

impl TransferData {
    pub(in crate::driver::active::flow) fn new(
        data: &mut Data,
        id: socket::Id,
        send_data: &mut ArrayVec<u8, 254>,
    ) -> Self {
        data.command = Command::TransferData;
        data.data = unsafe {
            ArrayVec::try_from_iter(iter::once(id.0).chain(send_data.iter().copied()))
                .unwrap_unchecked()
        };
        send_data.clear();

        Self { id }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) enum Error {
    SocketId {
        received: socket::Id,
        expected: socket::Id,
    },
    Empty(Command),
    Payload(super::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::SocketId { received, expected } => write!(
                formatter,
                "received socket ID {received}, but expected socket ID {expected}",
            ),
            Self::Empty(command) => write!(
                formatter,
                "received length of 0 for {command} packet, but expected nonzero length"
            ),
            Self::Payload(_) => formatter.write_str("payload error"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::SocketId { .. } => None,
            Self::Empty(_) => None,
            Self::Payload(error) => Some(error),
        }
    }
}

impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        Self::Payload(error)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::driver::active::flow) enum Response {
    Data,
    FinalData,
    ConnectionFailed,
}

impl Payload for TransferData {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::TransferData | Command::ConnectionClosed => {
                if let Some(&byte) = data.data.as_slice().first() {
                    let socket_id = byte.into();
                    if socket_id == self.id {
                        if matches!(data.command, Command::TransferData) {
                            Ok(Response::Data)
                        } else {
                            Ok(Response::FinalData)
                        }
                    } else {
                        Err(Error::SocketId {
                            received: socket_id,
                            expected: self.id,
                        })
                    }
                } else {
                    Err(Error::Empty(data.command))
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::TransferData(
                        command::error::transfer_data::Error::CommunicationFailed,
                    ) => Ok(Response::ConnectionFailed),
                    _ => Err(super::Error::UnexpectedCommandError(error).into()),
                }
            }
            unexpected => Err(super::Error::UnsupportedCommand {
                received: unexpected,
                expected: &[
                    Command::TransferData,
                    Command::ConnectionClosed,
                    Command::CommandError,
                ],
            }
            .into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Payload},
        Error, Response, TransferData,
    };
    use crate::{
        ArrayVec,
        driver::{Command, active::flow::packet::payload, command},
        socket,
    };
    use alloc::format;
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();
        let id = socket::Id(0x42);
        let mut send_data = assert_ok!(ArrayVec::try_from_iter(*b"send data"));

        let payload = TransferData::new(&mut data, id, &mut send_data);

        assert_eq!(payload.id, id);
        assert!(send_data.is_empty());
        assert_eq!(data.command, Command::TransferData);
        assert_eq!(
            data.data.as_slice(),
            [0x42, b's', b'e', b'n', b'd', b' ', b'd', b'a', b't', b'a']
        );
    }

    #[test]
    fn parse_data() {
        let data = Data {
            command: Command::TransferData,
            data: assert_ok!(ArrayVec::try_from_iter([
                0x42, b'r', b'e', b'c', b'e', b'i', b'v', b'e', b' ', b'd', b'a', b't', b'a'
            ])),
        };

        let payload = TransferData {
            id: socket::Id(0x42),
        };

        assert_ok_eq!(payload.parse(&data), Response::Data);
    }

    #[test]
    fn parse_final_data() {
        let data = Data {
            command: Command::ConnectionClosed,
            data: assert_ok!(ArrayVec::try_from_iter([
                0x42, b'r', b'e', b'c', b'e', b'i', b'v', b'e', b' ', b'd', b'a', b't', b'a'
            ])),
        };

        let payload = TransferData {
            id: socket::Id(0x42),
        };

        assert_ok_eq!(payload.parse(&data), Response::FinalData);
    }

    #[test]
    fn parse_connection_failed() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::TransferData as u8,
                command::error::transfer_data::Error::CommunicationFailed as u8,
            ])),
        };

        let payload = TransferData {
            id: socket::Id(0x42),
        };

        assert_ok_eq!(payload.parse(&data), Response::ConnectionFailed);
    }

    #[test]
    fn parse_error_socket_id() {
        let data = Data {
            command: Command::TransferData,
            data: assert_ok!(ArrayVec::try_from_iter([1])),
        };

        let payload = TransferData { id: socket::Id(2) };

        assert_err_eq!(
            payload.parse(&data),
            Error::SocketId {
                received: socket::Id(1),
                expected: socket::Id(2)
            }
        );
    }

    #[test]
    fn parse_error_empty() {
        let data = Data {
            command: Command::TransferData,
            data: ArrayVec::new(),
        };

        let payload = TransferData { id: socket::Id(0) };

        assert_err_eq!(payload.parse(&data), Error::Empty(Command::TransferData));
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::TransferData as u8,
                command::error::transfer_data::Error::NotConnected as u8,
            ])),
        };

        let payload = TransferData {
            id: socket::Id(0x42),
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnexpectedCommandError(
                command::Error::TransferData(command::error::transfer_data::Error::NotConnected)
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = TransferData { id: socket::Id(0) };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[
                    Command::TransferData,
                    Command::ConnectionClosed,
                    Command::CommandError
                ]
            })
        );
    }

    #[test]
    fn error_socket_id_display() {
        assert_eq!(
            format!(
                "{}",
                Error::SocketId {
                    received: socket::Id(0x54),
                    expected: socket::Id(0x1a)
                }
            ),
            "received socket ID 0x54, but expected socket ID 0x1a",
        );
    }

    #[test]
    fn error_empty_display() {
        assert_eq!(
            format!("{}", Error::Empty(Command::ConnectionClosed)),
            "received length of 0 for Connection Closed (0x1f) packet, but expected nonzero length",
        );
    }
}
