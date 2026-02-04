pub(crate) const TM0CNT: *mut Control = 0x0400_0102 as *mut Control;
pub(crate) const TM1CNT: *mut Control = 0x0400_0106 as *mut Control;
pub(crate) const TM2CNT: *mut Control = 0x0400_010a as *mut Control;
pub(crate) const TM3CNT: *mut Control = 0x0400_010e as *mut Control;
pub(crate) const TM0VAL: *mut u16 = 0x0400_0100 as *mut u16;
pub(crate) const TM1VAL: *mut u16 = 0x0400_0104 as *mut u16;
pub(crate) const TM2VAL: *mut u16 = 0x0400_0108 as *mut u16;
pub(crate) const TM3VAL: *mut u16 = 0x0400_010c as *mut u16;

/// The frequency with which to increment the timer.
#[derive(Debug)]
pub(crate) enum Frequency {
    /// A single period is approximately 61us.
    _1024 = 3,
}

#[derive(Debug)]
pub(crate) struct Control(u16);

impl Control {
    pub(crate) fn new() -> Self {
        Self(0)
    }

    pub(crate) fn frequency(self, frequency: Frequency) -> Self {
        Self((self.0 & 0b1111_1111_1111_1100) | (frequency as u16))
    }

    pub(crate) fn interrupts(self, enable: bool) -> Self {
        Self((self.0 & 0b1111_1111_1011_1111) | ((enable as u16) << 6))
    }

    pub(crate) fn start(self, start: bool) -> Self {
        Self((self.0 & 0b1111_1111_0111_1111) | ((start as u16) << 7))
    }
}
