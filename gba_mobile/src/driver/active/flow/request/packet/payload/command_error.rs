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

#[cfg(test)]
mod tests {
    use super::{super::Error, parse};
    use crate::{
        ArrayVec,
        driver::{Command, command},
    };
    use claims::{assert_err_eq, assert_ok, assert_ok_eq};
    use gba_test::test;

    #[test]
    fn parse_valid_error() {
        assert_ok_eq!(
            parse(&assert_ok!(ArrayVec::try_from_iter([
                Command::BeginSession as u8,
                command::error::begin_session::Error::AlreadyActive as u8
            ]))),
            command::Error::BeginSession(command::error::begin_session::Error::AlreadyActive)
        );
    }

    #[test]
    fn parse_invalid_error_command() {
        assert_err_eq!(
            parse(&assert_ok!(ArrayVec::try_from_iter([0, 1]))),
            Error::UnknownCommandError(command::error::Unknown::UnknownCommand {
                unknown: command::Unknown(0),
                error: 1
            })
        );
    }

    #[test]
    fn parse_invalid_error_reason() {
        assert_err_eq!(
            parse(&assert_ok!(ArrayVec::try_from_iter([
                Command::BeginSession as u8,
                0xff
            ]))),
            Error::UnknownCommandError(command::error::Unknown::BeginSession(
                command::error::UnknownError(0xff)
            ))
        );
    }
}
