use core::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub(in crate::driver) enum Protocol {
    Tcp,
    Udp,
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum Failure {
    Dns,
    Connect,
    ConnectionFailed,
}

impl Display for Failure {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Dns => formatter.write_str("DNS query failed"),
            Self::Connect => formatter.write_str("failed to connect"),
            Self::ConnectionFailed => formatter.write_str("the connection failed"),
        }
    }
}

impl core::error::Error for Failure {}
