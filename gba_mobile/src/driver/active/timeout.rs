use super::flow;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Flow(flow::Timeout),
    Queue,
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Flow(_) => formatter.write_str("timeout while processing active request flow"),
            Self::Queue => formatter
                .write_str("timeout while waiting for new request to be added to the queue"),
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Flow(timeout) => Some(timeout),
            Self::Queue => None,
        }
    }
}

impl From<flow::Timeout> for Timeout {
    fn from(timeout: flow::Timeout) -> Self {
        Self::Flow(timeout)
    }
}
