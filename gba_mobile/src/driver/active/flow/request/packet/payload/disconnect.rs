use super::{super::Data, Error, Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Disconnect {
    _private: PhantomData<()>,
}

impl Disconnect {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::HangUpTelephone;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for Disconnect {
    type Response<'a> = ();
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::HangUpTelephone => {
                if data.data.is_empty() {
                    Ok(())
                } else {
                    Err(Error::InvalidLength {
                        command: Command::HangUpTelephone,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::HangUpTelephone(
                        command::error::hang_up_telephone::Error::NotConnected,
                    ) => Ok(()),
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::HangUpTelephone, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        Disconnect,
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

        Disconnect::new(&mut data);

        assert_eq!(
            data,
            Data {
                command: Command::HangUpTelephone,
                data: ArrayVec::new(),
            }
        );
    }

    #[test]
    fn parse_ok() {
        let data = Data {
            command: Command::HangUpTelephone,
            data: ArrayVec::new(),
        };

        let payload = Disconnect {
            _private: PhantomData,
        };

        assert_ok!(payload.parse(&data));
    }

    #[test]
    fn parse_ok_already_disconnected() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::HangUpTelephone as u8,
                command::error::hang_up_telephone::Error::NotConnected as u8,
            ])),
        };

        let payload = Disconnect {
            _private: PhantomData,
        };

        assert_ok!(payload.parse(&data));
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::HangUpTelephone,
            data: assert_ok!(ArrayVec::try_from_iter([1, 2])),
        };

        let payload = Disconnect {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::HangUpTelephone,
                received: 2,
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

        let payload = Disconnect {
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

        let payload = Disconnect {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::HangUpTelephone, Command::CommandError]
            }
        );
    }
}
