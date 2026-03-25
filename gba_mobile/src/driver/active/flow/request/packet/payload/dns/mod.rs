pub(in crate::driver) mod error;

use super::{Payload, addr, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::{net::Ipv4Addr, num::NonZeroU16};
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct Dns {
    domain_name: ArrayVec<u8, 255>,
}

impl Dns {
    pub(in crate::driver::active::flow) fn new(domain_name: ArrayVec<u8, 255>) -> Self {
        Self { domain_name }
    }
}

impl Payload for Dns {
    type Send = Self;

    type ReceiveCommand = ReceiveCommand;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for Dns {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> crate::driver::Command {
        Command::DnsQuery
    }

    fn length(&self) -> u8 {
        self.domain_name.len()
    }

    fn get(&self, index: u8) -> u8 {
        self.domain_name.get(index).copied().unwrap_or(0x00)
    }

    fn finish(self) -> Self::ReceiveCommand {
        ReceiveCommand
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveCommand;

impl super::ReceiveCommand for ReceiveCommand {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::DnsQuery => Ok(ReceiveLength::DnsQuery),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveLength {
    DnsQuery,
    CommandError,
}

impl super::ReceiveLength for ReceiveLength {
    type ReceiveCommand = ReceiveCommand;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidLength;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>
    {
        match self {
            Self::DnsQuery => {
                if length == 4 {
                    Ok(Either::Left(ReceiveData::DnsQuery(addr::Data::new())))
                } else {
                    Err((error::InvalidLength::DnsQuery(length), ReceiveCommand))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData::CommandError(
                        command_error::Data::new(),
                    )))
                } else {
                    Err((error::InvalidLength::CommandError(length), ReceiveCommand))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveData {
    DnsQuery(addr::Data),
    CommandError(command_error::Data),
}

impl super::ReceiveData for ReceiveData {
    type ReceiveCommand = ReceiveCommand;
    type ReceiveParsed = ReceiveParsed;
    type Error = error::InvalidData;

    fn receive_data(
        self,
        byte: u8,
    ) -> Result<
        Either<Self, Self::ReceiveParsed>,
        (Self::Error, Self::ReceiveCommand, Option<(NonZeroU16, u16)>),
    > {
        match self {
            Self::DnsQuery(data) => match data.receive_data(byte) {
                Either::Left(data) => Ok(Either::Left(Self::DnsQuery(data))),
                Either::Right(addr) => Ok(Either::Right(ReceiveParsed::Success(addr))),
            },
            Self::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self::CommandError(data))),
                Ok(Either::Right(command::Error::DnsQuery(
                    command::error::dns_query::Error::LookupFailed,
                ))) => Ok(Either::Right(ReceiveParsed::NotFound)),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    ReceiveCommand,
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    ReceiveCommand,
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveParsed {
    Success(Ipv4Addr),
    NotFound,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        match self {
            Self::Success(_) => Command::DnsQuery,
            Self::NotFound => Command::CommandError,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand
    }
}
