use super::Error;
use crate::{
    ArrayVec,
    driver::{Command, command},
};

pub(super) fn parse(data: &ArrayVec<u8, 255>) -> Result<command::Error, Error> {
    let mut bytes = data.iter().copied();

    let command_byte = bytes.next().ok_or_else(|| Error::InvalidLength {
        command: Command::CommandError,
        received: 0,
        expected: 2,
    })?;
    let status_byte = bytes.next().ok_or_else(|| Error::InvalidLength {
        command: Command::CommandError,
        received: 1,
        expected: 2,
    })?;

    bytes
        .next()
        .map(|_| {
            Err(Error::InvalidLength {
                command: Command::CommandError,
                received: data.len(),
                expected: 2,
            })
        })
        .unwrap_or_else(|| Ok(()))?;

    command::Error::try_from((command_byte, status_byte)).map_err(Error::UnknownCommandError)
}
