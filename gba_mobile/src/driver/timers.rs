// These are at a rate of ~60us per tick.
pub(in crate::driver) const MICROSECONDS_200: u16 = u16::MIN.wrapping_sub(4);
pub(in crate::driver) const MICROSECONDS_400: u16 = u16::MIN.wrapping_sub(7);
