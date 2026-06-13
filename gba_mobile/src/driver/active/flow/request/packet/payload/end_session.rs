use super::{super::Data, Error, Payload, command_error};
use crate::{ArrayVec, driver::Command};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct EndSession {
    _private: PhantomData<()>,
}

impl EndSession {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::EndSession;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for EndSession {
    type Response<'a> = ();
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::EndSession => {
                if data.data.is_empty() {
                    Ok(())
                } else {
                    Err(Error::InvalidLength {
                        command: Command::EndSession,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => command_error::parse(&data.data)
                .and_then(|error| Err(Error::UnexpectedCommandError(error))),
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::EndSession, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        EndSession,
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

        EndSession::new(&mut data);

        assert_eq!(
            data,
            Data {
                command: Command::EndSession,
                data: ArrayVec::new(),
            }
        );
    }

    #[test]
    fn parse_ok() {
        let data = Data {
            command: Command::EndSession,
            data: ArrayVec::new(),
        };

        let payload = EndSession {
            _private: PhantomData,
        };

        assert_ok!(payload.parse(&data));
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::EndSession,
            data: assert_ok!(ArrayVec::try_from_iter([42])),
        };

        let payload = EndSession {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::EndSession,
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
                Command::EndSession as u8,
                command::error::end_session::Error::FailedToDisconnect as u8,
            ])),
        };

        let payload = EndSession {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnexpectedCommandError(command::Error::EndSession(
                command::error::end_session::Error::FailedToDisconnect
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = EndSession {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::EndSession, Command::CommandError]
            }
        );
    }
}
