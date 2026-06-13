use super::{super::Data, Error, Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct AcceptConnection {
    _private: PhantomData<()>,
}

impl AcceptConnection {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::WaitForTelephoneCall;
        data.data = ArrayVec::new();

        Self {
            _private: PhantomData,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::driver::active::flow) enum Response {
    Connected,
    NotConnected,
}

impl Payload for AcceptConnection {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::WaitForTelephoneCall => {
                if data.data.is_empty() {
                    Ok(Response::Connected)
                } else {
                    Err(Error::InvalidLength {
                        command: Command::WaitForTelephoneCall,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::WaitForTelephoneCall(
                        command::error::wait_for_telephone_call::Error::NoCallReceived,
                    ) => Ok(Response::NotConnected),
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::WaitForTelephoneCall, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        AcceptConnection, Response,
    };
    use crate::{
        ArrayVec,
        driver::{Command, command},
    };
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use core::marker::PhantomData;
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();

        AcceptConnection::new(&mut data);

        assert_eq!(
            data,
            Data {
                command: Command::WaitForTelephoneCall,
                data: ArrayVec::new(),
            }
        );
    }

    #[test]
    fn parse_connected() {
        let data = Data {
            command: Command::WaitForTelephoneCall,
            data: ArrayVec::new(),
        };

        let payload = AcceptConnection {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::Connected);
    }

    #[test]
    fn parse_not_connected() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::WaitForTelephoneCall as u8,
                command::error::wait_for_telephone_call::Error::NoCallReceived as u8
            ])),
        };

        let payload = AcceptConnection {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::NotConnected);
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::WaitForTelephoneCall,
            data: assert_ok!(ArrayVec::try_from_iter([0, 1, 2, 3])),
        };

        let payload = AcceptConnection {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::WaitForTelephoneCall,
                received: 4,
                expected: 0
            }
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::WaitForTelephoneCall as u8,
                command::error::wait_for_telephone_call::Error::InternalError as u8
            ])),
        };

        let payload = AcceptConnection {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnexpectedCommandError(command::Error::WaitForTelephoneCall(
                command::error::wait_for_telephone_call::Error::InternalError
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::InternalError,
            data: ArrayVec::new(),
        };

        let payload = AcceptConnection {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::InternalError,
                expected: &[Command::WaitForTelephoneCall, Command::CommandError]
            }
        );
    }
}
