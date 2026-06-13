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

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Payload},
        Error, ReadConfig,
    };
    use crate::{
        ArrayVec,
        config::format::Location,
        driver::{Command, active::flow::packet::payload, command},
    };
    use alloc::format;
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use deranged::RangedU8;
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();
        let location = Location {
            offset: 15,
            length: RangedU8::new_static::<64>(),
        };

        let payload = ReadConfig::new(&mut data, location);

        assert_eq!(payload.location, location);
        assert_eq!(data.command, Command::ReadConfigurationData);
        assert_eq!(data.data.as_slice(), [15, 64]);
    }

    #[test]
    fn parse_data() {
        let data = Data {
            command: Command::ReadConfigurationData,
            data: assert_ok!(ArrayVec::try_from_iter([
                42, b's', b'o', b'm', b'e', b' ', b'c', b'o', b'n', b'f', b'i', b'g', b'u', b'r',
                b'a', b't', b'i', b'o', b'n', b' ', b'd', b'a', b't', b'a',
            ])),
        };

        let payload = ReadConfig {
            location: Location {
                offset: 42,
                length: RangedU8::new_static::<23>(),
            },
        };

        assert_ok_eq!(payload.parse(&data), b"some configuration data");
    }

    #[test]
    fn parse_error_offset() {
        let data = Data {
            command: Command::ReadConfigurationData,
            data: assert_ok!(ArrayVec::try_from_iter([88, b'a'])),
        };

        let payload = ReadConfig {
            location: Location {
                offset: 0,
                length: RangedU8::new_static::<1>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Offset {
                received: 88,
                expected: 0
            }
        );
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::ReadConfigurationData,
            data: assert_ok!(ArrayVec::try_from_iter([88, b'a', b'b'])),
        };

        let payload = ReadConfig {
            location: Location {
                offset: 88,
                length: RangedU8::new_static::<1>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::InvalidLength {
                command: Command::ReadConfigurationData,
                received: 3,
                expected: 2
            })
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::ReadConfigurationData as u8,
                command::error::read_configuration_data::Error::ReadFailure as u8,
            ])),
        };

        let payload = ReadConfig {
            location: Location {
                offset: 0,
                length: RangedU8::new_static::<128>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnexpectedCommandError(
                command::Error::ReadConfigurationData(
                    command::error::read_configuration_data::Error::ReadFailure
                )
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = ReadConfig {
            location: Location {
                offset: 0,
                length: RangedU8::new_static::<128>(),
            },
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::Payload(payload::Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::ReadConfigurationData, Command::CommandError]
            })
        );
    }

    #[test]
    fn error_offset_display() {
        assert_eq!(
            format!(
                "{}",
                Error::Offset {
                    received: 24,
                    expected: 17
                },
            ),
            "received offset of 24, but expected offset of 17",
        )
    }
}
