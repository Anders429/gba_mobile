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

#[derive(Debug)]
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
