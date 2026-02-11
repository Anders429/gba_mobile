//! Commonly used frame counts.
//!
//! These are used when counting vblank frames to determine when certain things should happen, such
//! as idle pulses, timeouts, etc.

pub(in crate::driver) const ONE_SECOND: u8 = 60;
pub(in crate::driver) const THREE_SECONDS: u8 = 180;
pub(in crate::driver) const FIFTEEN_SECONDS: u16 = 900;

pub(in crate::driver) const ONE_HUNDRED_MILLISECONDS: u8 = 7;
