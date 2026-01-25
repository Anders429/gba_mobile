use crate::engine::{Command, HANDSHAKE, sink};

/// A data source.
///
/// This is the source of data when sending a given picket.
#[derive(Debug)]
pub(in crate::engine) enum Source {
    BeginSession,
}

impl Source {
    pub(in crate::engine) fn command(&self) -> Command {
        match self {
            Self::BeginSession => Command::BeginSession,
        }
    }

    pub(in crate::engine) fn length(&self) -> u8 {
        match self {
            Self::BeginSession => HANDSHAKE.len() as u8,
        }
    }

    pub(in crate::engine) fn get(&self, index: u8) -> u8 {
        match self {
            Self::BeginSession => HANDSHAKE.get(index as usize).copied().unwrap_or(0x00),
        }
    }

    pub(in crate::engine) fn sink(self) -> sink::Command {
        match self {
            Self::BeginSession => sink::Command::BeginSession,
        }
    }
}
