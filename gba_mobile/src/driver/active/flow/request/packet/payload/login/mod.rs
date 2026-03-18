pub(in crate::driver) mod data;
pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Command, command},
};
use core::{net::Ipv4Addr, num::NonZeroU16};
use data::Data;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct Login {
    id: ArrayVec<u8, 32>,
    password: ArrayVec<u8, 32>,
    primary_dns: Ipv4Addr,
    secondary_dns: Ipv4Addr,
}

impl Login {
    pub(in crate::driver::active::flow) fn new(
        id: ArrayVec<u8, 32>,
        password: ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Self {
        Self {
            id,
            password,
            primary_dns,
            secondary_dns,
        }
    }
}

impl Payload for Login {
    type Send = Self;

    type ReceiveCommand = ReceiveCommand;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for Login {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        Command::IspLogin
    }

    fn length(&self) -> u8 {
        // 1 byte id length + 1 byte password length + 4 bytes primary DNS + 4 bytes secondary DNS + actual login and password.
        self.id.len() + self.password.len() + 10
    }

    fn get(&self, index: u8) -> u8 {
        if let Some(id_index) = index.checked_sub(1) {
            if let Some(password_len_index) = id_index.checked_sub(self.id.len()) {
                if let Some(password_index) = password_len_index.checked_sub(1) {
                    if let Some(primary_dns_index) = password_index.checked_sub(self.password.len())
                    {
                        if let Some(secondary_dns_index) = primary_dns_index.checked_sub(4) {
                            self.secondary_dns
                                .octets()
                                .get(secondary_dns_index as usize)
                                .copied()
                                .unwrap_or(0x00)
                        } else {
                            self.primary_dns
                                .octets()
                                .get(primary_dns_index as usize)
                                .copied()
                                .unwrap_or(0x00)
                        }
                    } else {
                        self.password.get(password_index).copied().unwrap_or(0x00)
                    }
                } else {
                    self.password.len()
                }
            } else {
                self.id.get(id_index).copied().unwrap_or(0x00)
            }
        } else {
            self.id.len()
        }
    }

    fn finish(self) -> Self::ReceiveCommand {
        ReceiveCommand {
            primary_dns: self.primary_dns,
            secondary_dns: self.secondary_dns,
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveCommand {
    primary_dns: Ipv4Addr,
    secondary_dns: Ipv4Addr,
}

impl super::ReceiveCommand for ReceiveCommand {
    type ReceiveLength = ReceiveLength;
    type Error = error::UnsupportedCommand;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)> {
        match command {
            Command::IspLogin => Ok(ReceiveLength {
                received_command: ReceivedCommand::IspLogin,
                primary_dns: self.primary_dns,
                secondary_dns: self.secondary_dns,
            }),
            Command::CommandError => Ok(ReceiveLength {
                received_command: ReceivedCommand::CommandError,
                primary_dns: self.primary_dns,
                secondary_dns: self.secondary_dns,
            }),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
enum ReceivedCommand {
    IspLogin,
    CommandError,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveLength {
    received_command: ReceivedCommand,
    primary_dns: Ipv4Addr,
    secondary_dns: Ipv4Addr,
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
        match self.received_command {
            ReceivedCommand::IspLogin => {
                if length == 12 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::IspLogin(Data::new()),
                        primary_dns: self.primary_dns,
                        secondary_dns: self.secondary_dns,
                    }))
                } else {
                    Err((
                        error::InvalidLength::IspLogin(length),
                        ReceiveCommand {
                            primary_dns: self.primary_dns,
                            secondary_dns: self.secondary_dns,
                        },
                    ))
                }
            }
            ReceivedCommand::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData {
                        command_data: CommandData::CommandError(command_error::Data::new()),
                        primary_dns: self.primary_dns,
                        secondary_dns: self.secondary_dns,
                    }))
                } else {
                    Err((
                        error::InvalidLength::CommandError(length),
                        ReceiveCommand {
                            primary_dns: self.primary_dns,
                            secondary_dns: self.secondary_dns,
                        },
                    ))
                }
            }
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand {
            primary_dns: self.primary_dns,
            secondary_dns: self.secondary_dns,
        }
    }
}

#[derive(Debug)]
enum CommandData {
    IspLogin(Data),
    CommandError(command_error::Data),
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveData {
    command_data: CommandData,
    primary_dns: Ipv4Addr,
    secondary_dns: Ipv4Addr,
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
        match self.command_data {
            CommandData::IspLogin(data) => match data.receive_data(byte) {
                Either::Left(data) => Ok(Either::Left(Self {
                    command_data: CommandData::IspLogin(data),
                    primary_dns: self.primary_dns,
                    secondary_dns: self.secondary_dns,
                })),
                Either::Right(response) => Ok(Either::Right(ReceiveParsed {
                    response: Response::Connected {
                        ip: response.ip,
                        primary_dns: if response.primary_dns.is_unspecified() {
                            self.primary_dns
                        } else {
                            response.primary_dns
                        },
                        secondary_dns: if response.secondary_dns.is_unspecified() {
                            self.secondary_dns
                        } else {
                            response.secondary_dns
                        },
                    },
                    primary_dns: self.primary_dns,
                    secondary_dns: self.secondary_dns,
                })),
            },
            CommandData::CommandError(data) => match data.receive_data(byte) {
                Ok(Either::Left(data)) => Ok(Either::Left(Self {
                    command_data: CommandData::CommandError(data),
                    primary_dns: self.primary_dns,
                    secondary_dns: self.secondary_dns,
                })),
                Ok(Either::Right(command::Error::IspLogin(
                    command::error::isp_login::Error::NotInCall
                    | command::error::isp_login::Error::InternalError,
                ))) => Ok(Either::Right(ReceiveParsed {
                    response: Response::NotConnected,
                    primary_dns: self.primary_dns,
                    secondary_dns: self.secondary_dns,
                })),
                Ok(Either::Right(command_error)) => Err((
                    error::InvalidData::UnexpectedCommandError(command_error),
                    ReceiveCommand {
                        primary_dns: self.primary_dns,
                        secondary_dns: self.secondary_dns,
                    },
                    None,
                )),
                Err(error) => Err((
                    error::InvalidData::UnknownCommandError(error),
                    ReceiveCommand {
                        primary_dns: self.primary_dns,
                        secondary_dns: self.secondary_dns,
                    },
                    None,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum Response {
    Connected {
        ip: Ipv4Addr,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    },
    NotConnected,
}

#[derive(Debug)]
pub(in crate::driver) struct ReceiveParsed {
    pub(in crate::driver::active::flow) response: Response,
    primary_dns: Ipv4Addr,
    secondary_dns: Ipv4Addr,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        match self.response {
            Response::Connected { .. } => Command::IspLogin,
            Response::NotConnected => Command::CommandError,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand {
            primary_dns: self.primary_dns,
            secondary_dns: self.secondary_dns,
        }
    }
}
