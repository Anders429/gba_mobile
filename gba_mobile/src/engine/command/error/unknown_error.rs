use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::engine) struct UnknownError(pub(super) u8);

impl Display for UnknownError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "unknown error {:#04x}", self.0)
    }
}

impl core::error::Error for UnknownError {}
