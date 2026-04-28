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

#[derive(Debug)]
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
