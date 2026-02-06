pub(in crate::engine) mod error;

pub(in crate::engine) use error::Error;

use core::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::engine) struct Unknown(u8);

impl Display for Unknown {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "unknown command ID: {:#04x}", self.0)
    }
}

impl core::error::Error for Unknown {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::engine) enum Command {
    Empty = 0x0f,

    BeginSession = 0x10,
    EndSession = 0x11,
    DialTelephone = 0x12,
    HangUpTelephone = 0x13,
    WaitForTelephoneCall = 0x14,
    TransferData = 0x15,
    Reset = 0x16,
    TelephoneStatus = 0x17,

    Sio32Mode = 0x18,
    ReadConfigurationData = 0x19,
    WriteConfigurationData = 0x1a,

    ConnectionClosed = 0x1f,

    IspLogin = 0x21,
    IspLogout = 0x22,
    OpenTcpConnection = 0x23,
    CloseTcpConnection = 0x24,
    OpenUdpConnection = 0x25,
    CloseUdpConnection = 0x26,
    DnsQuery = 0x28,

    FirmwareVersion = 0x3f,

    CommandError = 0x6e,
    NotSupportedError = 0x70,
    MalformedError = 0x71,
    InternalError = 0x72,
}

impl Display for Command {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("Empty (0x0f)"),
            Self::BeginSession => formatter.write_str("Begin Session (0x10)"),
            Self::EndSession => formatter.write_str("End Session (0x11)"),
            Self::DialTelephone => formatter.write_str("Dial Telephone (0x12)"),
            Self::HangUpTelephone => formatter.write_str("Hang Up Telephone (0x13)"),
            Self::WaitForTelephoneCall => formatter.write_str("Wait For Telephone Call (0x14)"),
            Self::TransferData => formatter.write_str("Transfer Data (0x15)"),
            Self::Reset => formatter.write_str("Reset (0x16)"),
            Self::TelephoneStatus => formatter.write_str("Telephone Status (0x17)"),
            Self::Sio32Mode => formatter.write_str("SIO32 Mode (0x18)"),
            Self::ReadConfigurationData => formatter.write_str("Read Configuration Data (0x19)"),
            Self::WriteConfigurationData => formatter.write_str("Write Configuration Data (0x1a)"),
            Self::ConnectionClosed => formatter.write_str("Connection Closed (0x1f)"),
            Self::IspLogin => formatter.write_str("ISP Login (0x21)"),
            Self::IspLogout => formatter.write_str("ISP Logout (0x22)"),
            Self::OpenTcpConnection => formatter.write_str("Open TCP Connection (0x23)"),
            Self::CloseTcpConnection => formatter.write_str("Close TCP Connection (0x24)"),
            Self::OpenUdpConnection => formatter.write_str("Open UDP Connection (0x25)"),
            Self::CloseUdpConnection => formatter.write_str("Close UDP Connection (0x26)"),
            Self::DnsQuery => formatter.write_str("DNS Query (0x28)"),
            Self::FirmwareVersion => formatter.write_str("Firmware Version (0x3f)"),
            Self::CommandError => formatter.write_str("Command Error (0x6e)"),
            Self::NotSupportedError => formatter.write_str("Not Supported Error (0x70)"),
            Self::MalformedError => formatter.write_str("Malformed Error (0x71)"),
            Self::InternalError => formatter.write_str("Internal Error (0x72)"),
        }
    }
}

impl TryFrom<u8> for Command {
    type Error = Unknown;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x0f => Ok(Self::Empty),
            0x10 => Ok(Self::BeginSession),
            0x11 => Ok(Self::EndSession),
            0x12 => Ok(Self::DialTelephone),
            0x13 => Ok(Self::HangUpTelephone),
            0x14 => Ok(Self::WaitForTelephoneCall),
            0x15 => Ok(Self::TransferData),
            0x16 => Ok(Self::Reset),
            0x17 => Ok(Self::TelephoneStatus),
            0x18 => Ok(Self::Sio32Mode),
            0x19 => Ok(Self::ReadConfigurationData),
            0x1a => Ok(Self::WriteConfigurationData),
            0x1f => Ok(Self::ConnectionClosed),
            0x21 => Ok(Self::IspLogin),
            0x22 => Ok(Self::IspLogout),
            0x23 => Ok(Self::OpenTcpConnection),
            0x24 => Ok(Self::CloseTcpConnection),
            0x25 => Ok(Self::OpenUdpConnection),
            0x26 => Ok(Self::CloseUdpConnection),
            0x28 => Ok(Self::DnsQuery),
            0x3f => Ok(Self::FirmwareVersion),
            0x6e => Ok(Self::CommandError),
            0x70 => Ok(Self::NotSupportedError),
            0x71 => Ok(Self::MalformedError),
            0x72 => Ok(Self::InternalError),
            _ => Err(Unknown(byte)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, Unknown};
    use claims::{assert_err_eq, assert_ok_eq};
    use gba_test::test;

    #[test]
    fn from_valid_byte() {
        assert_ok_eq!(Command::try_from(0x24), Command::CloseTcpConnection);
    }

    #[test]
    fn from_unknown_byte() {
        assert_err_eq!(Command::try_from(0xff), Unknown(0xff));
    }
}
