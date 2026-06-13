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

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Payload},
        Error, WriteConfig,
    };
    use crate::{
        ArrayVec,
        config::format::Location,
        driver::{Command, active::flow::packet::payload, command},
    };
    use claims::{assert_err_eq, assert_ok};
    use deranged::RangedU8;
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();
        let location = Location {
            offset: 15,
            length: RangedU8::new_static::<24>(),
        };
        let mut segment = [0; 128];
        segment[..24].copy_from_slice(b"write configuration data");

        let payload = WriteConfig::new(&mut data, location, &segment);

        assert_eq!(payload.location, location);
        assert_eq!(data.command, Command::WriteConfigurationData);
        assert_eq!(
            data.data.as_slice(),
            [
                15, b'w', b'r', b'i', b't', b'e', b' ', b'c', b'o', b'n', b'f', b'i', b'g', b'u',
                b'r', b'a', b't', b'i', b'o', b'n', b' ', b'd', b'a', b't', b'a'
            ]
        );
    }

    #[test]
    fn parse_ok() {
        let data = Data {
            command: Command::WriteConfigurationData,
            data: assert_ok!(ArrayVec::try_from_iter([0, 96])),
        };

        let payload = WriteConfig {
            location: Location {
                offset: 0,
                length: RangedU8::new_static::<96>(),
            },
        };

        assert_ok!(payload.parse(&data));
    }

    #[test]
    fn parse_error_offset() {
        let data = Data {
            command: Command::WriteConfigurationData,
            data: assert_ok!(ArrayVec::try_from_iter([0, 96])),
        };

        let payload = WriteConfig {
            location: Location {
                offset: 22,
                length: RangedU8::new_static::<96>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Offset {
                received: 0,
                expected: 22
            }
        );
    }

    #[test]
    fn parse_error_length() {
        let data = Data {
            command: Command::WriteConfigurationData,
            data: assert_ok!(ArrayVec::try_from_iter([12, 54])),
        };

        let payload = WriteConfig {
            location: Location {
                offset: 12,
                length: RangedU8::new_static::<96>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Length {
                received: 54,
                expected: 96
            }
        );
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::WriteConfigurationData,
            data: assert_ok!(ArrayVec::try_from_iter([0])),
        };

        let payload = WriteConfig {
            location: Location {
                offset: 12,
                length: RangedU8::new_static::<96>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::InvalidLength {
                command: Command::WriteConfigurationData,
                received: 1,
                expected: 2
            })
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::WriteConfigurationData as u8,
                command::error::write_configuration_data::Error::WriteFailure as u8,
            ])),
        };

        let payload = WriteConfig {
            location: Location {
                offset: 27,
                length: RangedU8::new_static::<128>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnexpectedCommandError(
                command::Error::WriteConfigurationData(
                    command::error::write_configuration_data::Error::WriteFailure
                )
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: assert_ok!(ArrayVec::try_from_iter([0, 96])),
        };

        let payload = WriteConfig {
            location: Location {
                offset: 0,
                length: RangedU8::new_static::<96>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::WriteConfigurationData, Command::CommandError]
            })
        );
    }
}
