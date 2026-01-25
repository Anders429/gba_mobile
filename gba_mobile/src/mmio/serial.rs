pub(crate) const SIODATA32: *mut u32 = 0x0400_0120 as *mut u32;
pub(crate) const SIOCNT: *mut Control = 0x0400_0128 as *mut Control;
pub(crate) const SIODATA8: *mut u8 = 0x0400_012a as *mut u8;
pub(crate) const RCNT: *mut Mode = 0x0400_0134 as *mut Mode;

/// Serial mode selection.
#[derive(Debug)]
pub(crate) struct Mode(#[allow(dead_code)] u16);

impl Mode {
    pub(crate) const NORMAL: Self = Self(0b0000_0000_0000_0000);
}

/// The length of data being transferred.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(crate) enum TransferLength {
    _8Bit = 0,
    _32Bit = 1,
}

/// Serial control.
#[derive(Debug)]
pub(crate) struct Control(u16);

impl Control {
    pub(crate) const fn new() -> Self {
        Self(0)
    }

    /// Configures the transfer length.
    pub(crate) const fn transfer_length(self, transfer_length: TransferLength) -> Self {
        Self((self.0 & 0b1100_1111_1111_1111) | ((transfer_length as u16) << 12))
    }
}
