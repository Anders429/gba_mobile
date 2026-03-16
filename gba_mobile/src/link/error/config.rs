use crate::{driver, link};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub struct Error<ConfigError> {
    kind: Kind<ConfigError>,
}

impl<ConfigError> Error<ConfigError> {
    pub(crate) fn config_error(config_error: ConfigError) -> Self {
        Self {
            kind: Kind::Config(config_error),
        }
    }
}

impl<ConfigError> Display for Error<ConfigError> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl<ConfigError> core::error::Error for Error<ConfigError>
where
    ConfigError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<ConfigError> From<driver::error::link::Error> for Error<ConfigError> {
    fn from(error: driver::error::link::Error) -> Self {
        Self {
            kind: Kind::Link(error.into()),
        }
    }
}

#[derive(Debug)]
enum Kind<ConfigError> {
    Link(link::Error),
    Config(ConfigError),
}

impl<ConfigError> Display for Kind<ConfigError> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Link(_) => formatter.write_str("Mobile Adapter link connection error"),
            Self::Config(_) => formatter.write_str("config parse error"),
        }
    }
}

impl<ConfigError> core::error::Error for Kind<ConfigError>
where
    ConfigError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Link(error) => Some(error),
            Self::Config(error) => Some(error),
        }
    }
}
