use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::engine) enum Timeout {
    Serial,
    Response,
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Serial => formatter.write_str("the adapter did not send another byte"),
            Self::Response => formatter.write_str("the adapter did not send a response packet"),
        }
    }
}

impl core::error::Error for Timeout {}
