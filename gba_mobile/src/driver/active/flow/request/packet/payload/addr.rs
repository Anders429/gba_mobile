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
