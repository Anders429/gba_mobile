use super::{
    accept, close_tcp, close_udp, connect, disconnect, dns, end, idle, login, open_tcp, open_udp,
    reset, start, status, transfer_data, write_config,
};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Start(start::Timeout),
    End(end::Timeout),
    Reset(reset::Timeout),
    Accept(accept::Timeout),
    Connect(connect::Timeout),
    Login(login::Timeout),
    Disconnect(disconnect::Timeout),
    OpenTcp(open_tcp::Timeout),
    OpenUdp(open_udp::Timeout),
    CloseTcp(close_tcp::Timeout),
    CloseUdp(close_udp::Timeout),
    TransferData(transfer_data::Timeout),
    Dns(dns::Timeout),
    WriteConfig(write_config::Timeout),
    Status(status::Timeout),
    Idle(idle::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start(_) => formatter.write_str("timeout during start"),
            Self::End(_) => formatter.write_str("timeout during end"),
            Self::Reset(_) => formatter.write_str("timeout during reset"),
            Self::Accept(_) => formatter.write_str("timeout during accept"),
            Self::Connect(_) => formatter.write_str("timeout during connect"),
            Self::Login(_) => formatter.write_str("timeout during login"),
            Self::Disconnect(_) => formatter.write_str("timeout during disconnect"),
            Self::OpenTcp(_) => formatter.write_str("timeout during open tcp"),
            Self::OpenUdp(_) => formatter.write_str("timeout during open udp"),
            Self::CloseTcp(_) => formatter.write_str("timeout during close tcp"),
            Self::CloseUdp(_) => formatter.write_str("timeout during close udp"),
            Self::TransferData(_) => formatter.write_str("timeout during transfer data"),
            Self::Dns(_) => formatter.write_str("timeout during dns"),
            Self::WriteConfig(_) => formatter.write_str("timeout during write config"),
            Self::Status(_) => formatter.write_str("timeout during status"),
            Self::Idle(_) => formatter.write_str("timeout during idle"),
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Start(timeout) => Some(timeout),
            Self::End(timeout) => Some(timeout),
            Self::Reset(timeout) => Some(timeout),
            Self::Accept(timeout) => Some(timeout),
            Self::Connect(timeout) => Some(timeout),
            Self::Login(timeout) => Some(timeout),
            Self::Disconnect(timeout) => Some(timeout),
            Self::OpenTcp(timeout) => Some(timeout),
            Self::OpenUdp(timeout) => Some(timeout),
            Self::CloseTcp(timeout) => Some(timeout),
            Self::CloseUdp(timeout) => Some(timeout),
            Self::TransferData(timeout) => Some(timeout),
            Self::Dns(timeout) => Some(timeout),
            Self::WriteConfig(timeout) => Some(timeout),
            Self::Status(timeout) => Some(timeout),
            Self::Idle(timeout) => Some(timeout),
        }
    }
}
