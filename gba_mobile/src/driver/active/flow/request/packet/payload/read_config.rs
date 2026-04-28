use super::{super::Data, Payload, command_error};
use crate::{ArrayVec, config::format::Location, driver::Command};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct ReadConfig {
    location: Location,
}

impl ReadConfig {
    pub(in crate::driver::active::flow) fn new(data: &mut Data, location: Location) -> Self {
        data.command = Command::ReadConfigurationData;
        data.data = unsafe {
            ArrayVec::try_from_iter([location.offset, location.length.get()]).unwrap_unchecked()
        };

        Self { location }
    }
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Offset { received: u8, expected: u8 },
    Payload(super::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Offset { received, expected } => write!(
                formatter,
                "received offset of {received}, but expected offset of {expected}",
            ),
            Self::Payload(_) => formatter.write_str("payload error"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Offset { .. } => None,
            Self::Payload(error) => Some(error),
        }
    }
}

impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        Self::Payload(error)
    }
}

impl Payload for ReadConfig {
    type Response<'a> = &'a [u8];
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::ReadConfigurationData => {
                if let Some((&offset, slice)) = data.data.as_slice().split_first() {
                    if offset == self.location.offset {
                        if slice.len() == (self.location.length.get()) as usize {
                            Ok(slice)
                        } else {
                            Err(super::Error::InvalidLength {
                                command: Command::ReadConfigurationData,
                                received: slice.len() as u8 + 1,
                                expected: self.location.length.get() + 1,
                            }
                            .into())
                        }
                    } else {
                        Err(Error::Offset {
                            received: offset,
                            expected: self.location.offset,
                        })
                    }
                } else {
                    Err(super::Error::InvalidLength {
                        command: Command::ReadConfigurationData,
                        received: 0,
                        expected: self.location.length.get() + 1,
                    }
                    .into())
                }
            }
            Command::CommandError => command_error::parse(&data.data)
                .map_err(Into::into)
                .and_then(|error| Err(super::Error::UnexpectedCommandError(error).into())),
            unexpected => Err(super::Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::ReadConfigurationData, Command::CommandError],
            }
            .into()),
        }
    }
}
