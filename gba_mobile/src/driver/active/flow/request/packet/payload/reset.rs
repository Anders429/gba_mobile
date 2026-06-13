use super::{super::Data, Error, Payload, command_error};
use crate::{ArrayVec, driver::Command};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Reset {
    _private: PhantomData<()>,
}

impl Reset {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::Reset;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for Reset {
    type Response<'a> = ();
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::Reset => {
                if data.data.is_empty() {
                    Ok(())
                } else {
                    Err(Error::InvalidLength {
                        command: Command::Reset,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => command_error::parse(&data.data)
                .and_then(|error| Err(Error::UnexpectedCommandError(error))),
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::Reset, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        Reset,
    };
    use crate::{
        ArrayVec,
        driver::{Command, command},
    };
    use claims::{assert_err_eq, assert_ok};
    use core::marker::PhantomData;
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();

        Reset::new(&mut data);

        assert_eq!(
            data,
            Data {
                command: Command::Reset,
                data: ArrayVec::new(),
            }
        );
    }

    #[test]
    fn parse_ok() {
        let data = Data {
            command: Command::Reset,
            data: ArrayVec::new(),
        };

        let payload = Reset {
            _private: PhantomData,
        };

        assert_ok!(payload.parse(&data));
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::Reset,
            data: assert_ok!(ArrayVec::try_from_iter([0xff])),
        };

        let payload = Reset {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::Reset,
                received: 1,
                expected: 0
            }
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::Reset as u8,
                command::error::reset::Error::FailedToDisconnect as u8,
            ])),
        };

        let payload = Reset {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnexpectedCommandError(command::Error::Reset(
                command::error::reset::Error::FailedToDisconnect
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = Reset {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::Reset, Command::CommandError]
            }
        );
    }
}
