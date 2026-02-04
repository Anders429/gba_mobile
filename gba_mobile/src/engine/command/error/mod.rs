pub(in crate::engine) mod begin_session;
pub(in crate::engine) mod close_tcp_connection;
pub(in crate::engine) mod close_udp_connection;
pub(in crate::engine) mod dial_telephone;
pub(in crate::engine) mod dns_query;
pub(in crate::engine) mod end_session;
pub(in crate::engine) mod hang_up_telephone;
pub(in crate::engine) mod isp_login;
pub(in crate::engine) mod isp_logout;
pub(in crate::engine) mod open_tcp_connection;
pub(in crate::engine) mod open_udp_connection;
pub(in crate::engine) mod read_configuration_data;
pub(in crate::engine) mod reset;
pub(in crate::engine) mod sio32_mode;
pub(in crate::engine) mod transfer_data;
pub(in crate::engine) mod unknown_error;
pub(in crate::engine) mod wait_for_telephone_call;
pub(in crate::engine) mod write_configuration_data;

use super::Command;
use core::{
    fmt,
    fmt::{Display, Formatter},
};
use unknown_error::UnknownError;

#[derive(Debug, Eq, PartialEq)]
pub(in crate::engine) enum Unknown {
    Empty(UnknownError),
    BeginSession(UnknownError),
    EndSession(UnknownError),
    DialTelephone(UnknownError),
    HangUpTelephone(UnknownError),
    WaitForTelephoneCall(UnknownError),
    TransferData(UnknownError),
    Reset(UnknownError),
    TelephoneStatus(UnknownError),
    Sio32Mode(UnknownError),
    ReadConfigurationData(UnknownError),
    WriteConfigurationData(UnknownError),
    ConnectionClosed(UnknownError),
    IspLogin(UnknownError),
    IspLogout(UnknownError),
    OpenTcpConnection(UnknownError),
    CloseTcpConnection(UnknownError),
    OpenUdpConnection(UnknownError),
    CloseUdpConnection(UnknownError),
    DnsQuery(UnknownError),
    FirmwareVersion(UnknownError),
    CommandError(UnknownError),
    NotSupportedError(UnknownError),
    MalformedError(UnknownError),
    InternalError(UnknownError),
    UnknownCommand { unknown: super::Unknown, error: u8 },
}

impl Unknown {
    fn fmt_for_command(formatter: &mut Formatter, command: Command) -> fmt::Result {
        write!(formatter, "command {command} failed with unknown error")
    }
}

impl Display for Unknown {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Empty(_) => Self::fmt_for_command(formatter, Command::Empty),
            Self::BeginSession(_) => Self::fmt_for_command(formatter, Command::BeginSession),
            Self::EndSession(_) => Self::fmt_for_command(formatter, Command::EndSession),
            Self::DialTelephone(_) => Self::fmt_for_command(formatter, Command::DialTelephone),
            Self::HangUpTelephone(_) => Self::fmt_for_command(formatter, Command::HangUpTelephone),
            Self::WaitForTelephoneCall(_) => {
                Self::fmt_for_command(formatter, Command::WaitForTelephoneCall)
            }
            Self::TransferData(_) => Self::fmt_for_command(formatter, Command::TransferData),
            Self::Reset(_) => Self::fmt_for_command(formatter, Command::Reset),
            Self::TelephoneStatus(_) => Self::fmt_for_command(formatter, Command::TelephoneStatus),
            Self::Sio32Mode(_) => Self::fmt_for_command(formatter, Command::Sio32Mode),
            Self::ReadConfigurationData(_) => {
                Self::fmt_for_command(formatter, Command::ReadConfigurationData)
            }
            Self::WriteConfigurationData(_) => {
                Self::fmt_for_command(formatter, Command::WriteConfigurationData)
            }
            Self::ConnectionClosed(_) => {
                Self::fmt_for_command(formatter, Command::ConnectionClosed)
            }
            Self::IspLogin(_) => Self::fmt_for_command(formatter, Command::IspLogin),
            Self::IspLogout(_) => Self::fmt_for_command(formatter, Command::IspLogout),
            Self::OpenTcpConnection(_) => {
                Self::fmt_for_command(formatter, Command::OpenTcpConnection)
            }
            Self::CloseTcpConnection(_) => {
                Self::fmt_for_command(formatter, Command::CloseTcpConnection)
            }
            Self::OpenUdpConnection(_) => {
                Self::fmt_for_command(formatter, Command::OpenUdpConnection)
            }
            Self::CloseUdpConnection(_) => {
                Self::fmt_for_command(formatter, Command::CloseUdpConnection)
            }
            Self::DnsQuery(_) => Self::fmt_for_command(formatter, Command::DnsQuery),
            Self::FirmwareVersion(_) => Self::fmt_for_command(formatter, Command::FirmwareVersion),
            Self::CommandError(_) => Self::fmt_for_command(formatter, Command::CommandError),
            Self::NotSupportedError(_) => {
                Self::fmt_for_command(formatter, Command::NotSupportedError)
            }
            Self::MalformedError(_) => Self::fmt_for_command(formatter, Command::MalformedError),
            Self::InternalError(_) => Self::fmt_for_command(formatter, Command::InternalError),

            Self::UnknownCommand { error, .. } => {
                write!(formatter, "unknown command failed with error {error:#04x}")
            }
        }
    }
}

impl core::error::Error for Unknown {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Empty(error) => Some(error),
            Self::BeginSession(error) => Some(error),
            Self::EndSession(error) => Some(error),
            Self::DialTelephone(error) => Some(error),
            Self::HangUpTelephone(error) => Some(error),
            Self::WaitForTelephoneCall(error) => Some(error),
            Self::TransferData(error) => Some(error),
            Self::Reset(error) => Some(error),
            Self::TelephoneStatus(error) => Some(error),
            Self::Sio32Mode(error) => Some(error),
            Self::ReadConfigurationData(error) => Some(error),
            Self::WriteConfigurationData(error) => Some(error),
            Self::ConnectionClosed(error) => Some(error),
            Self::IspLogin(error) => Some(error),
            Self::IspLogout(error) => Some(error),
            Self::OpenTcpConnection(error) => Some(error),
            Self::CloseTcpConnection(error) => Some(error),
            Self::OpenUdpConnection(error) => Some(error),
            Self::CloseUdpConnection(error) => Some(error),
            Self::DnsQuery(error) => Some(error),
            Self::FirmwareVersion(error) => Some(error),
            Self::CommandError(error) => Some(error),
            Self::NotSupportedError(error) => Some(error),
            Self::MalformedError(error) => Some(error),
            Self::InternalError(error) => Some(error),

            Self::UnknownCommand { unknown, .. } => Some(unknown),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(in crate::engine) enum Error {
    BeginSession(begin_session::Error),
    EndSession(end_session::Error),
    DialTelephone(dial_telephone::Error),
    HangUpTelephone(hang_up_telephone::Error),
    WaitForTelephoneCall(wait_for_telephone_call::Error),
    TransferData(transfer_data::Error),
    Reset(reset::Error),
    Sio32Mode(sio32_mode::Error),
    ReadConfigurationData(read_configuration_data::Error),
    WriteConfigurationData(write_configuration_data::Error),
    IspLogin(isp_login::Error),
    IspLogout(isp_logout::Error),
    OpenTcpConnection(open_tcp_connection::Error),
    CloseTcpConnection(close_tcp_connection::Error),
    OpenUdpConnection(open_udp_connection::Error),
    CloseUdpConnection(close_udp_connection::Error),
    DnsQuery(dns_query::Error),
}

impl Error {
    fn fmt_for_command(formatter: &mut Formatter, command: Command) -> fmt::Result {
        write!(formatter, "command {command} failed")
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::BeginSession(_) => Self::fmt_for_command(formatter, Command::BeginSession),
            Self::EndSession(_) => Self::fmt_for_command(formatter, Command::EndSession),
            Self::DialTelephone(_) => Self::fmt_for_command(formatter, Command::DialTelephone),
            Self::HangUpTelephone(_) => Self::fmt_for_command(formatter, Command::HangUpTelephone),
            Self::WaitForTelephoneCall(_) => {
                Self::fmt_for_command(formatter, Command::WaitForTelephoneCall)
            }
            Self::TransferData(_) => Self::fmt_for_command(formatter, Command::TransferData),
            Self::Reset(_) => Self::fmt_for_command(formatter, Command::Reset),
            Self::Sio32Mode(_) => Self::fmt_for_command(formatter, Command::Sio32Mode),
            Self::ReadConfigurationData(_) => {
                Self::fmt_for_command(formatter, Command::ReadConfigurationData)
            }
            Self::WriteConfigurationData(_) => {
                Self::fmt_for_command(formatter, Command::WriteConfigurationData)
            }
            Self::IspLogin(_) => Self::fmt_for_command(formatter, Command::IspLogin),
            Self::IspLogout(_) => Self::fmt_for_command(formatter, Command::IspLogout),
            Self::OpenTcpConnection(_) => {
                Self::fmt_for_command(formatter, Command::OpenTcpConnection)
            }
            Self::CloseTcpConnection(_) => {
                Self::fmt_for_command(formatter, Command::CloseTcpConnection)
            }
            Self::OpenUdpConnection(_) => {
                Self::fmt_for_command(formatter, Command::OpenUdpConnection)
            }
            Self::CloseUdpConnection(_) => {
                Self::fmt_for_command(formatter, Command::CloseUdpConnection)
            }
            Self::DnsQuery(_) => Self::fmt_for_command(formatter, Command::DnsQuery),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::BeginSession(error) => Some(error),
            Self::EndSession(error) => Some(error),
            Self::DialTelephone(error) => Some(error),
            Self::HangUpTelephone(error) => Some(error),
            Self::WaitForTelephoneCall(error) => Some(error),
            Self::TransferData(error) => Some(error),
            Self::Reset(error) => Some(error),
            Self::Sio32Mode(error) => Some(error),
            Self::ReadConfigurationData(error) => Some(error),
            Self::WriteConfigurationData(error) => Some(error),
            Self::IspLogin(error) => Some(error),
            Self::IspLogout(error) => Some(error),
            Self::OpenTcpConnection(error) => Some(error),
            Self::CloseTcpConnection(error) => Some(error),
            Self::OpenUdpConnection(error) => Some(error),
            Self::CloseUdpConnection(error) => Some(error),
            Self::DnsQuery(error) => Some(error),
        }
    }
}

impl TryFrom<(u8, u8)> for Error {
    type Error = Unknown;

    fn try_from((command, error): (u8, u8)) -> Result<Self, Self::Error> {
        match command.try_into() {
            Ok(Command::Empty) => Err(Unknown::Empty(UnknownError(error))),
            Ok(Command::BeginSession) => error
                .try_into()
                .map(Self::BeginSession)
                .map_err(Unknown::BeginSession),
            Ok(Command::EndSession) => error
                .try_into()
                .map(Self::EndSession)
                .map_err(Unknown::EndSession),
            Ok(Command::DialTelephone) => error
                .try_into()
                .map(Self::DialTelephone)
                .map_err(Unknown::DialTelephone),
            Ok(Command::HangUpTelephone) => error
                .try_into()
                .map(Self::HangUpTelephone)
                .map_err(Unknown::HangUpTelephone),
            Ok(Command::WaitForTelephoneCall) => error
                .try_into()
                .map(Self::WaitForTelephoneCall)
                .map_err(Unknown::WaitForTelephoneCall),
            Ok(Command::TransferData) => error
                .try_into()
                .map(Self::TransferData)
                .map_err(Unknown::TransferData),
            Ok(Command::Reset) => error.try_into().map(Self::Reset).map_err(Unknown::Reset),
            Ok(Command::TelephoneStatus) => Err(Unknown::TelephoneStatus(UnknownError(error))),
            Ok(Command::Sio32Mode) => error
                .try_into()
                .map(Self::Sio32Mode)
                .map_err(Unknown::Sio32Mode),
            Ok(Command::ReadConfigurationData) => error
                .try_into()
                .map(Self::ReadConfigurationData)
                .map_err(Unknown::ReadConfigurationData),
            Ok(Command::WriteConfigurationData) => error
                .try_into()
                .map(Self::WriteConfigurationData)
                .map_err(Unknown::WriteConfigurationData),
            Ok(Command::ConnectionClosed) => Err(Unknown::ConnectionClosed(UnknownError(error))),
            Ok(Command::IspLogin) => error
                .try_into()
                .map(Self::IspLogin)
                .map_err(Unknown::IspLogin),
            Ok(Command::IspLogout) => error
                .try_into()
                .map(Self::IspLogout)
                .map_err(Unknown::IspLogout),
            Ok(Command::OpenTcpConnection) => error
                .try_into()
                .map(Self::OpenTcpConnection)
                .map_err(Unknown::OpenTcpConnection),
            Ok(Command::CloseTcpConnection) => error
                .try_into()
                .map(Self::CloseTcpConnection)
                .map_err(Unknown::CloseTcpConnection),
            Ok(Command::OpenUdpConnection) => error
                .try_into()
                .map(Self::OpenUdpConnection)
                .map_err(Unknown::OpenUdpConnection),
            Ok(Command::CloseUdpConnection) => error
                .try_into()
                .map(Self::CloseUdpConnection)
                .map_err(Unknown::CloseUdpConnection),
            Ok(Command::DnsQuery) => error
                .try_into()
                .map(Self::DnsQuery)
                .map_err(Unknown::DnsQuery),
            Ok(Command::FirmwareVersion) => Err(Unknown::FirmwareVersion(UnknownError(error))),
            Ok(Command::CommandError) => Err(Unknown::CommandError(UnknownError(error))),
            Ok(Command::NotSupportedError) => Err(Unknown::NotSupportedError(UnknownError(error))),
            Ok(Command::MalformedError) => Err(Unknown::MalformedError(UnknownError(error))),
            Ok(Command::InternalError) => Err(Unknown::InternalError(UnknownError(error))),
            Err(unknown) => Err(Unknown::UnknownCommand { unknown, error }),
        }
    }
}
