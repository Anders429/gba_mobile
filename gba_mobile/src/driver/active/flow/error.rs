use super::{
    SocketSubFlow, accept, close_tcp, close_udp, connect, disconnect, dns, end, idle, login,
    open_tcp, open_udp, read_config, reset, start, status, transfer_data, write_config,
};
use crate::{
    config,
    driver::active::flow::{ConfigSubFlow, DnsSubFlow},
    socket,
};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

#[derive(Clone, Debug)]
pub(crate) enum Connection {
    Accept(accept::Error),
    Connect(connect::Error),
}

impl Display for Connection {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Accept(_) => formatter.write_str("error during accept"),
            Self::Connect(_) => formatter.write_str("error during connect"),
        }
    }
}

impl core::error::Error for Connection {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Accept(error) => Some(error),
            Self::Connect(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Socket<BufferError> {
    OpenTcp(open_tcp::Error),
    OpenUdp(open_udp::Error),
    CloseTcp(close_tcp::Error),
    CloseUdp(close_udp::Error),
    TransferData(transfer_data::Error<BufferError>),
}

impl<BufferError> Display for Socket<BufferError> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::OpenTcp(_) => formatter.write_str("error during open tcp"),
            Self::OpenUdp(_) => formatter.write_str("error during open udp"),
            Self::CloseTcp(_) => formatter.write_str("error during close tcp"),
            Self::CloseUdp(_) => formatter.write_str("error during close udp"),
            Self::TransferData(_) => formatter.write_str("error during transfer data"),
        }
    }
}

impl<BufferError> core::error::Error for Socket<BufferError>
where
    BufferError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::OpenTcp(error) => Some(error),
            Self::OpenUdp(error) => Some(error),
            Self::CloseTcp(error) => Some(error),
            Self::CloseUdp(error) => Some(error),
            Self::TransferData(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Dns<const MAX_LEN: usize> {
    Dns(dns::Error<MAX_LEN>),
}

impl<const MAX_LEN: usize> Display for Dns<MAX_LEN> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Dns(_) => formatter.write_str("error during dns"),
        }
    }
}

impl<const MAX_LEN: usize> core::error::Error for Dns<MAX_LEN> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Dns(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Config {
    ReadConfig(read_config::Error),
    WriteConfig(write_config::Error),
}

impl Display for Config {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::ReadConfig(_) => formatter.write_str("error during read config"),
            Self::WriteConfig(_) => formatter.write_str("error during write config"),
        }
    }
}

impl core::error::Error for Config {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ReadConfig(error) => Some(error),
            Self::WriteConfig(error) => Some(error),
        }
    }
}

pub(in crate::driver) enum Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    Start(start::Error),
    End(end::Error),
    Reset(reset::Error),
    Login(login::Error),
    Connection(<Socket1::ConnectionFlow as SocketSubFlow<Socket1>>::Error),
    Disconnect(disconnect::Error),
    Socket1(<Socket1::SocketFlow<0> as SocketSubFlow<Socket1>>::Error),
    Socket2(<Socket2::SocketFlow<1> as SocketSubFlow<Socket2>>::Error),
    Dns(<Dns::Flow as DnsSubFlow<Dns>>::Error),
    Config(<Config::Flow as ConfigSubFlow<Config>>::Error),
    Status(status::Error),
    Idle(idle::Error),
}

impl<Socket1, Socket2, Dns, Config> Clone for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn clone(&self) -> Self {
        match self {
            Self::Start(error) => Self::Start(error.clone()),
            Self::End(error) => Self::End(error.clone()),
            Self::Reset(error) => Self::Reset(error.clone()),
            Self::Login(error) => Self::Login(error.clone()),
            Self::Connection(error) => Self::Connection(error.clone()),
            Self::Disconnect(error) => Self::Disconnect(error.clone()),
            Self::Socket1(error) => Self::Socket1(error.clone()),
            Self::Socket2(error) => Self::Socket2(error.clone()),
            Self::Dns(error) => Self::Dns(error.clone()),
            Self::Config(error) => Self::Config(error.clone()),
            Self::Status(error) => Self::Status(error.clone()),
            Self::Idle(error) => Self::Idle(error.clone()),
        }
    }
}

impl<Socket1, Socket2, Dns, Config> Debug for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start(error) => formatter.debug_tuple("Start").field(error).finish(),
            Self::End(error) => formatter.debug_tuple("End").field(error).finish(),
            Self::Reset(error) => formatter.debug_tuple("Reset").field(error).finish(),
            Self::Login(error) => formatter.debug_tuple("Login").field(error).finish(),
            Self::Connection(error) => formatter.debug_tuple("Connection").field(error).finish(),
            Self::Disconnect(error) => formatter.debug_tuple("Disconnect").field(error).finish(),
            Self::Socket1(error) => formatter.debug_tuple("Socket1").field(error).finish(),
            Self::Socket2(error) => formatter.debug_tuple("Socket2").field(error).finish(),
            Self::Dns(error) => formatter.debug_tuple("Dns").field(error).finish(),
            Self::Config(error) => formatter.debug_tuple("Config").field(error).finish(),
            Self::Status(error) => formatter.debug_tuple("Status").field(error).finish(),
            Self::Idle(error) => formatter.debug_tuple("Idle").field(error).finish(),
        }
    }
}

impl<Socket1, Socket2, Dns, Config> Display for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start(_) => formatter.write_str("error during start"),
            Self::End(_) => formatter.write_str("error during end"),
            Self::Reset(_) => formatter.write_str("error during reset"),
            Self::Login(_) => formatter.write_str("error during login"),
            Self::Connection(_) => formatter.write_str("error during connection flow"),
            Self::Disconnect(_) => formatter.write_str("error during disconnect"),
            Self::Socket1(_) => formatter.write_str("error during socket 1 flow"),
            Self::Socket2(_) => formatter.write_str("error during socket 2 flow"),
            Self::Dns(_) => formatter.write_str("error during dns flow"),
            Self::Config(_) => formatter.write_str("error during config flow"),
            Self::Status(_) => formatter.write_str("error during status"),
            Self::Idle(_) => formatter.write_str("error during idle"),
        }
    }
}

impl<Socket1, Socket2, Dns, Config> core::error::Error for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Start(error) => Some(error),
            Self::End(error) => Some(error),
            Self::Reset(error) => Some(error),
            Self::Login(error) => Some(error),
            Self::Connection(error) => Some(error),
            Self::Disconnect(error) => Some(error),
            Self::Socket1(error) => Some(error),
            Self::Socket2(error) => Some(error),
            Self::Dns(error) => Some(error),
            Self::Config(error) => Some(error),
            Self::Status(error) => Some(error),
            Self::Idle(error) => Some(error),
        }
    }
}
