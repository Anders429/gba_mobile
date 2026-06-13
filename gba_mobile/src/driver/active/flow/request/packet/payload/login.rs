use super::{super::Data, Error, Payload, addr, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::{iter, marker::PhantomData, net::Ipv4Addr};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Login {
    _private: PhantomData<()>,
}

impl Login {
    pub(in crate::driver::active::flow) fn new(
        data: &mut Data,
        id: &ArrayVec<u8, 32>,
        password: &ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Self {
        data.command = Command::PppLogin;
        data.data = unsafe {
            ArrayVec::try_from_iter(
                iter::once(id.len())
                    .chain(id.iter().copied())
                    .chain(iter::once(password.len()))
                    .chain(password.iter().copied())
                    .chain(primary_dns.octets().into_iter())
                    .chain(secondary_dns.octets().into_iter()),
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
    Connected {
        ip: Ipv4Addr,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    },
    NotConnected,
}

impl Payload for Login {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::PppLogin => {
                let mut bytes = data.data.iter().copied();
                let ip = addr::parse(&mut bytes, Command::PppLogin, 0, 12)?;
                let primary_dns = addr::parse(&mut bytes, Command::PppLogin, 4, 12)?;
                let secondary_dns = addr::parse(&mut bytes, Command::PppLogin, 8, 12)?;
                Ok(Response::Connected {
                    ip,
                    primary_dns,
                    secondary_dns,
                })
            }
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::PppLogin(
                        command::error::ppp_login::Error::NotInCall
                        | command::error::ppp_login::Error::InternalError,
                    ) => Ok(Response::NotConnected),
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::PppLogin, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        Login, Response,
    };
    use crate::{
        ArrayVec,
        driver::{Command, command},
    };
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use core::{marker::PhantomData, net::Ipv4Addr};
    use gba_test::test;

    #[test]
    fn new() {
        let mut data = Data::new();
        let id = assert_ok!(ArrayVec::try_from_iter(*b"AzureDiamond"));
        let password = assert_ok!(ArrayVec::try_from_iter(*b"hunter2"));
        let primary_dns = Ipv4Addr::from_octets([8, 8, 8, 8]);
        let secondary_dns = Ipv4Addr::from_octets([8, 8, 4, 4]);

        Login::new(&mut data, &id, &password, primary_dns, secondary_dns);

        assert_eq!(data.command, Command::PppLogin);
        assert_eq!(
            data.data.as_slice(),
            [
                12, // ID length.
                b'A', b'z', b'u', b'r', b'e', b'D', b'i', b'a', b'm', b'o', b'n', b'd', // ID.
                7,    // Password length.
                b'h', b'u', b'n', b't', b'e', b'r', b'2', // Password.
                8, 8, 8, 8, // Primary DNS.
                8, 8, 4, 4, // Secondary DNS.
            ]
        );
    }

    #[test]
    fn parse_connected() {
        let data = Data {
            command: Command::PppLogin,
            data: assert_ok!(ArrayVec::try_from_iter([
                127, 0, 0, 1, 8, 8, 8, 8, 8, 8, 4, 4
            ])),
        };

        let payload = Login {
            _private: PhantomData,
        };

        assert_ok_eq!(
            payload.parse(&data),
            Response::Connected {
                ip: Ipv4Addr::LOCALHOST,
                primary_dns: Ipv4Addr::from_octets([8, 8, 8, 8]),
                secondary_dns: Ipv4Addr::from_octets([8, 8, 4, 4])
            }
        );
    }

    #[test]
    fn parse_not_connected() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::PppLogin as u8,
                command::error::ppp_login::Error::NotInCall as u8,
            ])),
        };

        let payload = Login {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::NotConnected);
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::PppLogin,
            data: ArrayVec::new(),
        };

        let payload = Login {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::PppLogin,
                received: 0,
                expected: 12
            }
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::PppLogin as u8,
                command::error::ppp_login::Error::InvalidContents as u8,
            ])),
        };

        let payload = Login {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnexpectedCommandError(command::Error::PppLogin(
                command::error::ppp_login::Error::InvalidContents
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload = Login {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::PppLogin, Command::CommandError]
            }
        );
    }
}
