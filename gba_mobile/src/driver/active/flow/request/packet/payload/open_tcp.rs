use super::{super::Data, Error, Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
    socket,
};
use core::{marker::PhantomData, net::SocketAddrV4};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct OpenTcp {
    _private: PhantomData<()>,
}

impl OpenTcp {
    pub(in crate::driver::active::flow) fn new(data: &mut Data, socket: SocketAddrV4) -> Self {
        data.command = Command::OpenTcpConnection;
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

#[derive(Debug)]
pub(in crate::driver::active::flow) enum Response {
    Connected(socket::Id),
    NotConnected,
}

impl Payload for OpenTcp {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::OpenTcpConnection => {
                if data.data.len() == 1 {
                    Ok(Response::Connected(unsafe {
                        data.data.get(0).copied().unwrap_unchecked().into()
                    }))
                } else {
                    Err(Error::InvalidLength {
                        command: Command::OpenTcpConnection,
                        received: data.data.len(),
                        expected: 1,
                    })
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::OpenTcpConnection(
                        command::error::open_tcp_connection::Error::ConnectionFailed,
                    ) => Ok(Response::NotConnected),
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::OpenTcpConnection, Command::CommandError],
            }),
        }
    }
}
