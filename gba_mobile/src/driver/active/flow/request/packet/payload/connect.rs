use super::{super::Data, Error, Payload, command_error};
use crate::{
    Adapter, ArrayVec, Digit,
    driver::{Command, command},
};
use core::{iter, marker::PhantomData};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Connect {
    _private: PhantomData<()>,
}

impl Connect {
    pub(in crate::driver::active::flow) fn new(
        data: &mut Data,
        adapter: Adapter,
        digits: &ArrayVec<Digit, 32>,
    ) -> Self {
        data.command = Command::DialTelephone;
        data.data = unsafe {
            ArrayVec::try_from_iter(
                iter::once(adapter.dial_byte()).chain(digits.iter().map(|&digit| digit.into())),
            )
            .unwrap_unchecked()
        };

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

impl Payload for Connect {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::DialTelephone => {
                if data.data.is_empty() {
                    Ok(Response::Connected)
                } else {
                    Err(Error::InvalidLength {
                        command: Command::DialTelephone,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::DialTelephone(
                        command::error::dial_telephone::Error::LineBusy
                        | command::error::dial_telephone::Error::CommunicationFailed
                        | command::error::dial_telephone::Error::CallNotEstablished,
                    ) => Ok(Response::NotConnected),
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::DialTelephone, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        Connect, Response,
    };
    use crate::{
        Adapter, ArrayVec, Digit,
        driver::{Command, command},
    };
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use core::marker::PhantomData;
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();
        let adapter = Adapter::Green;
        let digits = assert_ok!(ArrayVec::try_from_iter([
            Digit::_8,
            Digit::_6,
            Digit::_7,
            Digit::_5,
            Digit::_3,
            Digit::_0,
            Digit::_9,
        ]));

        Connect::new(&mut data, adapter, &digits);

        assert_eq!(data.command, Command::DialTelephone);
        assert_eq!(
            data.data.as_slice(),
            [
                adapter.dial_byte(),
                b'8',
                b'6',
                b'7',
                b'5',
                b'3',
                b'0',
                b'9',
            ]
        );
    }

    #[test]
    fn parse_connected() {
        let data = Data {
            command: Command::DialTelephone,
            data: ArrayVec::new(),
        };

        let payload = Connect {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::Connected);
    }

    #[test]
    fn parse_not_connected() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::DialTelephone as u8,
                command::error::dial_telephone::Error::CallNotEstablished as u8,
            ])),
        };

        let payload = Connect {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::NotConnected);
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::DialTelephone,
            data: assert_ok!(ArrayVec::try_from_iter([1, 2, 3])),
        };

        let payload = Connect {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::DialTelephone,
                received: 3,
                expected: 0
            }
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::DialTelephone as u8,
                command::error::dial_telephone::Error::InvalidContents as u8,
            ])),
        };

        let payload = Connect {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnexpectedCommandError(command::Error::DialTelephone(
                command::error::dial_telephone::Error::InvalidContents
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = Connect {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::DialTelephone, Command::CommandError]
            }
        );
    }
}
