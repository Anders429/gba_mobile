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

#[derive(Clone, Debug)]
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

#[derive(Debug)]
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
