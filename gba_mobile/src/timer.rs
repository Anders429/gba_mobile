use crate::mmio::timer::{TM0CNT, TM1CNT, TM2CNT, TM3CNT};

/// Identifier for a timer on the GBA.
#[derive(Clone, Copy, Debug)]
pub enum Timer {
    /// Timer 0.
    _0,
    /// Timer 1.
    _1,
    /// Timer 2.
    _2,
    /// Timer 3.
    _3,
}

impl Timer {
    pub(crate) fn stop(self) {
        match self {
            Self::_0 => unsafe { TM0CNT.write_volatile(TM0CNT.read_volatile().start(false)) },
            Self::_1 => unsafe { TM1CNT.write_volatile(TM1CNT.read_volatile().start(false)) },
            Self::_2 => unsafe { TM2CNT.write_volatile(TM2CNT.read_volatile().start(false)) },
            Self::_3 => unsafe { TM3CNT.write_volatile(TM3CNT.read_volatile().start(false)) },
        }
    }
}
