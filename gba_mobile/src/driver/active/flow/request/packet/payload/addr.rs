use super::Error;
use crate::driver::Command;
use core::net::Ipv4Addr;

pub(super) fn parse<Bytes>(
    mut bytes: Bytes,
    command: Command,
    byte_offset: u8,
    expected: u8,
) -> Result<Ipv4Addr, Error>
where
    Bytes: Iterator<Item = u8>,
{
    let octet1 = bytes.next().ok_or_else(|| Error::InvalidLength {
        command,
        received: byte_offset,
        expected,
    })?;
    let octet2 = bytes.next().ok_or_else(|| Error::InvalidLength {
        command,
        received: byte_offset + 1,
        expected,
    })?;
    let octet3 = bytes.next().ok_or_else(|| Error::InvalidLength {
        command,
        received: byte_offset + 2,
        expected,
    })?;
    let octet4 = bytes.next().ok_or_else(|| Error::InvalidLength {
        command,
        received: byte_offset + 3,
        expected,
    })?;

    Ok(Ipv4Addr::from_octets([octet1, octet2, octet3, octet4]))
}

#[cfg(test)]
mod tests {
    use super::{super::Error, parse};
    use crate::driver::Command;
    use claims::{assert_err_eq, assert_ok_eq};
    use core::net::Ipv4Addr;
    use gba_test::test;

    #[test]
    fn parse_valid() {
        assert_ok_eq!(
            parse([127, 0, 0, 1].into_iter(), Command::Empty, 0, 4),
            Ipv4Addr::LOCALHOST,
        );
    }

    #[test]
    fn parse_invalid_0_bytes() {
        assert_err_eq!(
            parse([].into_iter(), Command::Empty, 0, 4),
            Error::InvalidLength {
                command: Command::Empty,
                received: 0,
                expected: 4,
            },
        );
    }

    #[test]
    fn parse_invalid_1_byte() {
        assert_err_eq!(
            parse([127].into_iter(), Command::Empty, 1, 5),
            Error::InvalidLength {
                command: Command::Empty,
                received: 2,
                expected: 5,
            },
        );
    }

    #[test]
    fn parse_invalid_2_bytes() {
        assert_err_eq!(
            parse([127, 0].into_iter(), Command::Empty, 2, 42),
            Error::InvalidLength {
                command: Command::Empty,
                received: 4,
                expected: 42,
            },
        );
    }

    #[test]
    fn parse_invalid_3_bytes() {
        assert_err_eq!(
            parse([127, 0, 0].into_iter(), Command::Empty, 100, 255),
            Error::InvalidLength {
                command: Command::Empty,
                received: 103,
                expected: 255,
            },
        );
    }
}
