use crate::driver::{Command, command};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) enum Error {
    UnsupportedCommand {
        received: Command,
        expected: &'static [Command],
    },
    InvalidLength {
        command: Command,
        received: u8,
        expected: u8,
    },
    UnknownCommandError(command::error::Unknown),
    UnexpectedCommandError(command::Error),
}

fn fmt_unsupported_command(
    formatter: &mut Formatter,
    received: Command,
    expected: &'static [Command],
) -> fmt::Result {
    write!(formatter, "unsupported command {received}")?;
    if let Some((last, list)) = expected.split_last() {
        if list.len() <= 1 {
            if let Some(first) = list.first() {
                write!(formatter, "; supported commands are {first} and {last}")?;
            } else {
                write!(formatter, "; expected {last}")?;
            }
        } else {
            formatter.write_str("; supported commands are ")?;
            for command in list {
                write!(formatter, "{command}, ")?;
            }
            write!(formatter, "and {last}")?;
        }
    }

    Ok(())
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnsupportedCommand { received, expected } => {
                fmt_unsupported_command(formatter, *received, expected)
            }
            Self::InvalidLength {
                command,
                received,
                expected,
            } => write!(
                formatter,
                "received length of {received} for {command} packet, but expected length of {expected}"
            ),
            Self::UnknownCommandError(_) => {
                formatter.write_str("unable to parse command error payload")
            }
            Self::UnexpectedCommandError(_) => {
                formatter.write_str("received unexpected command error")
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::UnsupportedCommand { .. } => None,
            Self::InvalidLength { .. } => None,
            Self::UnknownCommandError(unknown) => Some(unknown),
            Self::UnexpectedCommandError(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::driver::{Command, command};
    use alloc::format;
    use gba_test::test;

    #[test]
    fn display_unsupported_command_empty_expected() {
        assert_eq!(
            format!(
                "{}",
                Error::UnsupportedCommand {
                    received: Command::Empty,
                    expected: &[]
                }
            ),
            "unsupported command Empty (0x0f)"
        )
    }

    #[test]
    fn display_unsupported_command_one_expected() {
        assert_eq!(
            format!(
                "{}",
                Error::UnsupportedCommand {
                    received: Command::BeginSession,
                    expected: &[Command::EndSession]
                }
            ),
            "unsupported command Begin Session (0x10); expected End Session (0x11)"
        )
    }

    #[test]
    fn display_unsupported_command_two_expected() {
        assert_eq!(
            format!(
                "{}",
                Error::UnsupportedCommand {
                    received: Command::WriteConfigurationData,
                    expected: &[Command::ReadConfigurationData, Command::CommandError]
                }
            ),
            "unsupported command Write Configuration Data (0x1a); supported commands are Read Configuration Data (0x19) and Command Error (0x6e)"
        )
    }

    #[test]
    fn display_unsupported_command_three_expected() {
        assert_eq!(
            format!(
                "{}",
                Error::UnsupportedCommand {
                    received: Command::FirmwareVersion,
                    expected: &[
                        Command::CloseTcpConnection,
                        Command::CommandError,
                        Command::NotSupportedError
                    ]
                }
            ),
            "unsupported command Firmware Version (0x3f); supported commands are Close TCP Connection (0x24), Command Error (0x6e), and Not Supported Error (0x70)"
        )
    }

    #[test]
    fn display_invalid_length() {
        assert_eq!(
            format!(
                "{}",
                Error::InvalidLength {
                    command: Command::PppLogin,
                    received: 12,
                    expected: 126
                },
            ),
            "received length of 12 for PPP Login (0x21) packet, but expected length of 126"
        )
    }

    #[test]
    fn display_unknown_command_error() {
        assert_eq!(
            format!(
                "{}",
                Error::UnknownCommandError(command::error::Unknown::Empty(
                    command::error::UnknownError(0xff)
                )),
            ),
            "unable to parse command error payload",
        );
    }

    #[test]
    fn display_unexpected_command_error() {
        assert_eq!(
            format!(
                "{}",
                Error::UnexpectedCommandError(command::Error::DnsQuery(
                    command::error::dns_query::Error::NotLoggedIn
                )),
            ),
            "received unexpected command error",
        )
    }
}
