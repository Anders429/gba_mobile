use crate::driver;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub struct P2p<IoError> {
    internal: driver::error::connection_io::Error<IoError>,
}

impl<IoError> Display for P2p<IoError>
where
    IoError: Display,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.internal.fmt(formatter)
    }
}

impl<IoError> core::error::Error for P2p<IoError>
where
    IoError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<IoError> From<driver::error::connection_io::Error<IoError>> for P2p<IoError> {
    fn from(error: driver::error::connection_io::Error<IoError>) -> Self {
        Self { internal: error }
    }
}

#[derive(Debug)]
pub struct Socket<IoError> {
    internal: driver::error::socket_io::Error<IoError>,
}

impl<IoError> Display for Socket<IoError>
where
    IoError: Display,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.internal.fmt(formatter)
    }
}

impl<IoError> core::error::Error for Socket<IoError>
where
    IoError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<IoError> From<driver::error::socket_io::Error<IoError>> for Socket<IoError> {
    fn from(error: driver::error::socket_io::Error<IoError>) -> Self {
        Self { internal: error }
    }
}
