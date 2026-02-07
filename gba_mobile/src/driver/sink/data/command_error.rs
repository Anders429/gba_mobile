use crate::driver::command;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) enum Data {
    Command,
    Status(u8),
}

impl Data {
    pub(in crate::driver) fn parse(
        self,
        byte: u8,
    ) -> Result<Either<Self, command::Error>, (command::error::Unknown, u16)> {
        match self {
            Self::Command => Ok(Either::Left(Self::Status(byte))),
            Self::Status(command_byte) => match command::Error::try_from((command_byte, byte)) {
                Ok(command_error) => Ok(Either::Right(command_error)),
                Err(unknown) => Err((unknown, 1)),
            },
        }
    }
}
