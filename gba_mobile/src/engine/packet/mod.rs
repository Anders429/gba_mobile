pub(in crate::engine) mod receive;
pub(in crate::engine) mod send;

mod error;

pub(in crate::engine) use error::Error;

use super::{Command, Source};

pub(in crate::engine) const MAX_RETRIES: u8 = 5;

/// In-progress packet communication.
#[derive(Debug)]
pub(in crate::engine) enum Packet {
    /// Sending in SIO8 mode.
    Send8 {
        step: send::Step8,
        source: Source,
        checksum: u16,

        attempt: u8,
    },
    /// Sending in SIO32 mode.
    Send32 {
        step: send::Step32,
        source: Source,
        checksum: u16,

        attempt: u8,
    },
    /// Receiving in SIO8 mode.
    Receive8 {
        step: receive::Step8,
        checksum: u16,

        attempt: u8,
    },
    /// Receiving in SIO32 mode.
    Receive32 {
        step: receive::Step32,
        checksum: u16,

        attempt: u8,
    },
    /// Receiving in SIO8 mode while in an error state.
    Receive8Error {
        step: receive::Step8Error,

        error: receive::Error,
        attempt: u8,
    },
    /// Receiving in SIO32 mode while in an error state.
    Receive32Error {
        step: receive::Step32Error,

        error: receive::Error,
        attempt: u8,
    },
}
