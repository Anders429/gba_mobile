use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(crate) struct Error {
    kind: Kind,
}

impl Error {
    pub(in crate::driver) fn superseded() -> Self {
        Self {
            kind: Kind::Superseded,
        }
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        Self {
            kind: Kind::Driver(error),
        }
    }
}

#[derive(Debug)]
enum Kind {
    Driver(super::Error),
    Superseded,
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Driver(_) => formatter.write_str("the driver is in an error state"),
            Self::Superseded => formatter.write_str("the link connection was superseded"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Driver(error) => Some(error),
            Self::Superseded => None,
        }
    }
}
