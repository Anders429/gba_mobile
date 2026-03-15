use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) struct Timeout;

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("timeout while waiting for adapter to enter idle state")
    }
}

impl core::error::Error for Timeout {}
