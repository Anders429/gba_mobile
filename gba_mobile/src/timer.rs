use crate::mmio::timer::{TM0CNT, TM1CNT, TM2CNT, TM3CNT};

#[derive(Clone, Copy, Debug)]
pub enum Timer {
    _0,
    _1,
    _2,
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
