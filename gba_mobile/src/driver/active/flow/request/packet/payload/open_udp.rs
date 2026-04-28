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
