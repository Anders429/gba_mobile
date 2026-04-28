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

#[derive(Debug)]
pub(in crate::driver::active::flow) enum Response {
    BeginSession,
    AlreadyActive,
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Handshake { byte: u8, index: usize },
    Payload(super::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Handshake { byte, index } => write!(
                formatter,
                "unexpected byte {byte:#04x} at index {index}; expected {:04x}",
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
