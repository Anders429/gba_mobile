pub(in crate::driver) mod error;

use super::{Payload, command_error};
use crate::{
    ArrayVec,
    driver::{Adapter, Command, command},
    phone_number::Digit,
};
use core::num::NonZeroU16;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct Connect {
    adapter: Adapter,
    phone_number: ArrayVec<Digit, 32>,
}

impl Connect {
    pub(in crate::driver::active::flow) fn new(
        adapter: Adapter,
        phone_number: ArrayVec<Digit, 32>,
    ) -> Self {
        Self {
            adapter,
            phone_number,
        }
    }
}

impl Payload for Connect {
    type Send = Self;

    type ReceiveCommand = ReceiveCommand;
    type ReceiveLength = ReceiveLength;
    type ReceiveData = ReceiveData;
    type ReceiveParsed = ReceiveParsed;
}

impl super::Send for Connect {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        Command::DialTelephone
    }

    fn length(&self) -> u8 {
        self.phone_number.len() + 1
    }

    fn get(&self, index: u8) -> u8 {
        if let Some(phone_number_index) = index.checked_sub(1) {
            // Starting at index 1, we are returning the phone number bytes directly.
            self.phone_number
                .get(phone_number_index)
                .map(|&digit| digit.into())
                .unwrap_or(0x00)
        } else {
            // If this is the first byte, it is the dial byte for the adapter.
            self.adapter.dial_byte()
        }
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
            Command::DialTelephone => Ok(ReceiveLength::DialTelephone),
            Command::CommandError => Ok(ReceiveLength::CommandError),
            _ => Err((error::UnsupportedCommand(command), self)),
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveLength {
    DialTelephone,
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
            Self::DialTelephone => {
                if length == 0 {
                    Ok(Either::Right(ReceiveParsed::Connected))
                } else {
                    Err((error::InvalidLength::DialTelephone(length), ReceiveCommand))
                }
            }
            Self::CommandError => {
                if length == 2 {
                    Ok(Either::Left(ReceiveData(command_error::Data::new())))
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
pub(in crate::driver) struct ReceiveData(command_error::Data);

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
        match self.0.receive_data(byte) {
            Ok(Either::Left(data)) => Ok(Either::Left(Self(data))),
            Ok(Either::Right(command::Error::DialTelephone(
                command::error::dial_telephone::Error::LineBusy
                | command::error::dial_telephone::Error::CommunicationFailed
                | command::error::dial_telephone::Error::CallNotEstablished,
            ))) => Ok(Either::Right(ReceiveParsed::NotConnected)),
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
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum ReceiveParsed {
    Connected,
    NotConnected,
}

impl super::ReceiveParsed for ReceiveParsed {
    type ReceiveCommand = ReceiveCommand;

    fn command(&self) -> Command {
        match self {
            Self::Connected => Command::DialTelephone,
            Self::NotConnected => Command::CommandError,
        }
    }

    fn restart(self) -> Self::ReceiveCommand {
        ReceiveCommand
    }
}
