use super::{super::Data, Error, Payload, command_error};
use crate::{ArrayVec, driver::Command, socket};
use core::{marker::PhantomData, net::SocketAddrV4};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct OpenUdp {
    _private: PhantomData<()>,
}

impl OpenUdp {
    pub(in crate::driver::active::flow) fn new(data: &mut Data, socket: SocketAddrV4) -> Self {
        data.command = Command::OpenUdpConnection;
        data.data = unsafe {
            ArrayVec::try_from_iter(
                socket
                    .ip()
                    .octets()
                    .into_iter()
                    .chain(socket.port().to_be_bytes().into_iter()),
            )
            .unwrap_unchecked()
        };

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for OpenUdp {
    type Response<'a> = socket::Id;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::OpenUdpConnection => {
                if data.data.len() == 1 {
                    Ok(unsafe { data.data.get(0).copied().unwrap_unchecked().into() })
                } else {
                    Err(Error::InvalidLength {
                        command: Command::OpenUdpConnection,
                        received: data.data.len(),
                        expected: 1,
                    })
                }
            }
            Command::CommandError => command_error::parse(&data.data)
                .and_then(|error| Err(Error::UnexpectedCommandError(error))),
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::OpenUdpConnection, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        OpenUdp,
    };
    use crate::{
        ArrayVec,
        driver::{Command, command},
        socket,
    };
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use core::{
        marker::PhantomData,
        net::{Ipv4Addr, SocketAddrV4},
    };
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();
        let socket = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 80);

        OpenUdp::new(&mut data, socket);

        assert_eq!(
            data,
            Data {
                command: Command::OpenUdpConnection,
                data: assert_ok!(ArrayVec::try_from_iter([127, 0, 0, 1, 0, 80])),
            }
        );
    }

    #[test]
    fn parse_connected() {
        let data = Data {
            command: Command::OpenUdpConnection,
            data: assert_ok!(ArrayVec::try_from_iter([0x42])),
        };

        let payload = OpenUdp {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), socket::Id(0x42));
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::OpenUdpConnection,
            data: ArrayVec::new(),
        };

        let payload = OpenUdp {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::OpenUdpConnection,
                received: 0,
                expected: 1
            }
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::OpenUdpConnection as u8,
                command::error::open_udp_connection::Error::TooManyConnections as u8
            ])),
        };

        let payload = OpenUdp {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnexpectedCommandError(command::Error::OpenUdpConnection(
                command::error::open_udp_connection::Error::TooManyConnections
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = OpenUdp {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::OpenUdpConnection, Command::CommandError]
            }
        );
    }
}
