use crate::{
    engine::{Packet, Source},
    mmio::serial::TransferLength,
};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(in crate::engine) enum LinkingP2P {
    Waking,
    BeginSession,
    Sio32,
    WaitForIdle,
}

impl LinkingP2P {
    pub(in crate::engine) fn request(self, transfer_length: TransferLength) -> Packet {
        match self {
            Self::Waking => todo!(),
            Self::BeginSession => Packet::packet(transfer_length, Source::BeginSession),
            Self::Sio32 => Packet::packet(transfer_length, todo!()),
            Self::WaitForIdle => todo!(),
        }
    }

    pub(in crate::engine) fn next(self) -> Option<Self> {
        match self {
            Self::Waking => Some(Self::BeginSession),
            Self::BeginSession => Some(Self::Sio32),
            Self::Sio32 => Some(Self::WaitForIdle),
            Self::WaitForIdle => None,
        }
    }
}
