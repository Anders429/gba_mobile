use super::{super::Data, Error, Payload, addr, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::{marker::PhantomData, net::Ipv4Addr};

#[derive(Debug)]
pub(in crate::driver::active::flow) struct Dns<const MAX_LEN: usize> {
    _private: PhantomData<()>,
}

impl<const MAX_LEN: usize> Dns<MAX_LEN> {
    pub(in crate::driver::active::flow) fn new(
        data: &mut Data,
        name: &ArrayVec<u8, MAX_LEN>,
    ) -> Self {
        data.command = Command::DnsQuery;
        data.data =
            unsafe { ArrayVec::try_from_iter(name.iter().copied().take(255)).unwrap_unchecked() };

        Self {
            _private: PhantomData,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::driver::active::flow) enum Response {
    Success(Ipv4Addr),
    NotFound,
}

impl<const MAX_LEN: usize> Payload for Dns<MAX_LEN> {
    type Response<'a> = Response;
    type Error = Error;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error> {
        match data.command {
            Command::DnsQuery => addr::parse(data.data.iter().copied(), Command::DnsQuery, 0, 4)
                .map(|ip| Response::Success(ip)),
            Command::CommandError => {
                let error = command_error::parse(&data.data)?;
                match error {
                    command::Error::DnsQuery(command::error::dns_query::Error::LookupFailed) => {
                        Ok(Response::NotFound)
                    }
                    _ => Err(Error::UnexpectedCommandError(error)),
                }
            }
            unexpected => Err(Error::UnsupportedCommand {
                received: unexpected,
                expected: &[Command::DnsQuery, Command::CommandError],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Data, Error, Payload},
        Dns, Response,
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
        let name = assert_ok!(ArrayVec::<u8, 255>::try_from_iter(*b"example.com"));

        Dns::new(&mut data, &name);

        assert_eq!(data.command, Command::DnsQuery);
        assert_eq!(data.data, name);
    }

    #[test]
    fn parse_success() {
        let data = Data {
            command: Command::DnsQuery,
            data: assert_ok!(ArrayVec::try_from_iter([127, 0, 0, 1])),
        };

        let payload: Dns<255> = Dns {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::Success(Ipv4Addr::LOCALHOST));
    }

    #[test]
    fn parse_not_found() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::DnsQuery as u8,
                command::error::dns_query::Error::LookupFailed as u8,
            ])),
        };

        let payload: Dns<255> = Dns {
            _private: PhantomData,
        };

        assert_ok_eq!(payload.parse(&data), Response::NotFound);
    }

    #[test]
    fn parse_error_invalid_length() {
        let data = Data {
            command: Command::DnsQuery,
            data: ArrayVec::new(),
        };

        let payload: Dns<255> = Dns {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::InvalidLength {
                command: Command::DnsQuery,
                received: 0,
                expected: 4
            }
        );
    }

    #[test]
    fn parse_error_unexpected_error() {
        let data = Data {
            command: Command::CommandError,
            data: assert_ok!(ArrayVec::try_from_iter([
                Command::DnsQuery as u8,
                command::error::dns_query::Error::NotLoggedIn as u8,
            ])),
        };

        let payload: Dns<255> = Dns {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnexpectedCommandError(command::Error::DnsQuery(
                command::error::dns_query::Error::NotLoggedIn
            ))
        );
    }

    #[test]
    fn parse_error_unsupported_command() {
        let data = Data {
            command: Command::Empty,
            data: ArrayVec::new(),
        };

        let payload: Dns<255> = Dns {
            _private: PhantomData,
        };

        assert_err_eq!(
            payload.parse(&data),
            Error::UnsupportedCommand {
                received: Command::Empty,
                expected: &[Command::DnsQuery, Command::CommandError]
            }
        );
    }
}
