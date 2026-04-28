use super::{super::Data, Payload, command_error};
use crate::{ArrayVec, config::format::Location, driver::Command};
use core::{
    fmt::{self, Display, Formatter},
    iter,
};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct WriteConfig {
    location: Location,
}

impl WriteConfig {
    pub(in crate::driver::active::flow) fn new(
        data: &mut Data,
        location: Location,
        segment: &[u8; 128],
    ) -> Self {
        data.command = Command::WriteConfigurationData;
        data.data = unsafe {
            ArrayVec::try_from_iter(
                iter::once(location.offset)
                    .chain(segment.iter().copied().take(location.length.get() as usize)),
            )
            .unwrap_unchecked()
        };

        Self { location }
    }
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Offset { received: u8, expected: u8 },
    Length { received: u8, expected: u8 },
    Payload(super::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Offset { received, expected } => write!(
                formatter,
                "received offset of {received}, but expected offset of {expected}"
            ),
            Self::Length { received, expected } => write!(
                formatter,
                "received length of {received}, but expected length of {expected}"
            ),
            Self::Payload(_) => formatter.write_str("payload error"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Offset { .. } => None,
            Self::Length { .. } => None,
            Self::Payload(error) => Some(error),
        }
    }
}

impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        Self::Payload(error)
    }
}

impl Payload for WriteConfig {
    type Response<'a> = ();
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::WriteConfigurationData => {
                let mut bytes = data.data.iter();

                let offset = bytes
                    .next()
                    .copied()
                    .ok_or_else(|| super::Error::InvalidLength {
                        command: Command::WriteConfigurationData,
                        received: 0,
                        expected: 2,
                    })?;
                let length = bytes
                    .next()
                    .copied()
                    .ok_or_else(|| super::Error::InvalidLength {
                        command: Command::WriteConfigurationData,
                        received: 1,
                        expected: 2,
                    })?;

                bytes
                    .next()
                    .map(|_| {
                        Err::<(), Error>(
                            super::Error::InvalidLength {
                                command: Command::WriteConfigurationData,
                                received: data.data.len(),
                                expected: 2,
                            }
                            .into(),
                        )
                    })
                    .unwrap_or_else(|| Ok(()))?;

                if offset == self.location.offset {
                    if length == self.location.length.get() {
                        Ok(())
                    } else {
                        Err(Error::Length {
                            received: length,
                            expected: self.location.length.get(),
                        })
                    }
                } else {
                    Err(Error::Offset {
                        received: offset,
                        expected: self.location.offset,
                    })
                }
            }
            Command::CommandError => command_error::parse(&data.data)
                .map_err(Into::into)
                .and_then(|error| Err(super::Error::UnexpectedCommandError(error).into())),
            unexpected => Err(super::Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::WriteConfigurationData, Command::CommandError],
            }
            .into()),
        }
    }
}
