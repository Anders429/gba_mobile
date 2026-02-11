use crate::driver::{Command, HANDSHAKE, sink};

/// A data source.
///
/// This is the source of data when sending a given picket.
#[derive(Debug)]
pub(in crate::driver) enum Source {
    BeginSession,
    EnableSio32,

    WaitForCall,

    EndSession,
}

impl Source {
    pub(in crate::driver) fn command(&self) -> Command {
        match self {
            Self::BeginSession => Command::BeginSession,
            Self::EnableSio32 => Command::Sio32Mode,

            Self::WaitForCall => Command::WaitForTelephoneCall,

            Self::EndSession => Command::EndSession,
        }
    }

    pub(in crate::driver) fn length(&self) -> u8 {
        match self {
            Self::BeginSession => HANDSHAKE.len() as u8,
            Self::EnableSio32 => 1,

            Self::WaitForCall => 0,

            Self::EndSession => 0,
        }
    }

    // TODO: Should this be stateful instead? Like a `next()` function?
    pub(in crate::driver) fn get(&self, index: u8) -> u8 {
        match self {
            Self::BeginSession => HANDSHAKE.get(index as usize).copied().unwrap_or(0x00),
            Self::EnableSio32 => 0x01,

            Self::WaitForCall => 0x00,

            Self::EndSession => 0x00,
        }
    }

    pub(in crate::driver) fn sink(self) -> sink::Command {
        match self {
            Self::BeginSession => sink::Command::BeginSession,
            Self::EnableSio32 => sink::Command::EnableSio32,

            Self::WaitForCall => sink::Command::WaitForCall,

            Self::EndSession => sink::Command::EndSession,
        }
    }
}
