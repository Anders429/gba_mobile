use core::{
    convert::Infallible,
    fmt,
    fmt::{Display, Formatter},
    net::{Ipv4Addr, SocketAddrV4},
};

#[derive(Debug)]
pub enum Host<'a> {
    Ip(Ipv4Addr),
    Name(&'a [u8]),
}

pub trait ToSocket {
    type Error: core::error::Error + 'static;

    fn to_socket(&self) -> Result<(Host<'_>, u16), Self::Error>;
}

impl<T> ToSocket for &T
where
    T: ToSocket + ?Sized,
{
    type Error = T::Error;

    fn to_socket(&self) -> Result<(Host<'_>, u16), Self::Error> {
        (*self).to_socket()
    }
}

impl ToSocket for (Ipv4Addr, u16) {
    type Error = Infallible;

    fn to_socket(&self) -> Result<(Host<'_>, u16), Self::Error> {
        Ok((Host::Ip(self.0), self.1))
    }
}

impl ToSocket for SocketAddrV4 {
    type Error = Infallible;

    fn to_socket(&self) -> Result<(Host<'_>, u16), Self::Error> {
        Ok((Host::Ip(*self.ip()), self.port()))
    }
}

impl ToSocket for (&str, u16) {
    type Error = Infallible;

    fn to_socket(&self) -> Result<(Host<'_>, u16), Self::Error> {
        Ok((Host::Name(self.0.as_bytes()), self.1))
    }
}

#[derive(Debug)]
pub enum StrError {
    InvalidSocket,
    InvalidPort,
}

impl Display for StrError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::InvalidSocket => formatter.write_str("invalid socket address"),
            Self::InvalidPort => formatter.write_str("invalid port value"),
        }
    }
}

impl core::error::Error for StrError {}

impl ToSocket for str {
    type Error = StrError;

    fn to_socket(&self) -> Result<(Host<'_>, u16), Self::Error> {
        // Split host and port.
        let Some((host, port_str)) = self.rsplit_once(':') else {
            return Err(StrError::InvalidSocket);
        };
        // Parse the port.
        let Ok(port) = port_str.parse::<u16>() else {
            return Err(StrError::InvalidPort);
        };
        Ok((Host::Name(host.as_bytes()), port))
    }
}
