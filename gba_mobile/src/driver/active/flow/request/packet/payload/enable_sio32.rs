use super::{super::Data, Error, Payload, command_error};
use crate::{ArrayVec, driver::Command, mmio::serial::TransferLength};
use core::marker::PhantomData;

#[derive(Debug)]
pub(in crate::driver::active::flow) struct EnableSio32 {
    _private: PhantomData<()>,
}

impl EnableSio32 {
    pub(in crate::driver::active::flow) fn new(data: &mut Data) -> Self {
        data.command = Command::Sio32Mode;
        data.data = unsafe { ArrayVec::try_from_iter([0x01]).unwrap_unchecked() };

        Self {
            _private: PhantomData,
        }
    }
}

impl Payload for EnableSio32 {
    type Response<'a> = TransferLength;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::Sio32Mode => {
                if data.data.is_empty() {
                    Ok(TransferLength::_32Bit)
                } else {
                    Err(Error::InvalidLength {
                        command: Command::Sio32Mode,
                        received: data.data.len(),
                        expected: 0,
                    })
                }
            }
            Command::Reset => {
                if data.data.is_empty() {
                    Ok(TransferLength::_8Bit)
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
                expected: &[Command::Sio32Mode, Command::Reset, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        EnableSio32,
    };
    use crate::{
        ArrayVec,
        driver::{Command, command},
        mmio::serial::TransferLength,
    };
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use core::marker::PhantomData;
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();

        EnableSio32::new(&mut data);

        assert_eq!(data.command, Command::Sio32Mode);
        assert_eq!(data.data.as_slice(), [1]); // 1 to request to enable SIO32 mode.
    }

    #[test]
    fn parse_8_bit() {
        let data = Data {
            command: Command::Reset,
            data: ArrayVec::new(),
        };

        let payload = EnableSio32 {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), TransferLength::_8Bit);
    }

    #[test]
    fn parse_32_bit() {
        let data = Data {
            command: Command::Sio32Mode,
            data: ArrayVec::new(),
        };

        let payload = EnableSio32 {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), TransferLength::_32Bit);
    }

    #[test]
    fn parse_error_invalid_length_8_bit() {
        let data = Data {
            command: Command::Reset,
            data: assert_ok!(ArrayVec::try_from_iter([0])),
        };

        let payload = EnableSio32 {
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
    fn parse_error_invalid_length_32_bit() {
        let data = Data {
            command: Command::Sio32Mode,
            data: assert_ok!(ArrayVec::try_from_iter([1])),
        };

        let payload = EnableSio32 {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::Sio32Mode,
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
                Command::Sio32Mode as u8,
                command::error::sio32_mode::Error::InvalidContents as u8,
            ])),
        };

        let payload = EnableSio32 {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnexpectedCommandError(command::Error::Sio32Mode(
                command::error::sio32_mode::Error::InvalidContents
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = EnableSio32 {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::Sio32Mode, Command::Reset, Command::CommandError]
            }
        );
    }
}
