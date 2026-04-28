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
pub(in crate::driver::active::flow) struct CloseUdp {
    id: socket::Id,
}

impl CloseUdp {
    pub(in crate::driver::active::flow) fn new(data: &mut Data, id: socket::Id) -> Self {
        data.command = Command::CloseUdpConnection;
        data.data = unsafe { ArrayVec::try_from_iter([id.0]).unwrap_unchecked() };

        Self { id }
    }
}

#[derive(Debug)]
pub(in crate::driver::active::flow) enum Response {
    Closed,
    AlreadyClosed,
    AlreadyDisconnected,
}

#[derive(Clone, Debug)]
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

impl Payload for CloseUdp {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::CloseUdpConnection => {
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
                        command: Command::CloseUdpConnection,
                        received: data.data.len(),
                        expected: 1,
                    }
                    .into())
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::CloseUdpConnection(
                        command::error::close_udp_connection::Error::NotConnected,
                    ) => Ok(Response::AlreadyClosed),
                    command::Error::CloseUdpConnection(
                        command::error::close_udp_connection::Error::NotLoggedIn,
                    ) => Ok(Response::AlreadyDisconnected),
                    _ => Err(super::Error::UnexpectedCommandError(error).into()),
                }
            }
            unexpected => Err(super::Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::CloseUdpConnection, Command::CommandError],
            }
            .into()),
        }
    }
}
