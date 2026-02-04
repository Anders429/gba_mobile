use super::{Command, Finished};
use crate::{
    engine::{self, command},
    mmio::serial::TransferLength,
};

#[derive(Debug)]
pub(in crate::engine) enum Parsed {
    BeginSession,
    BeginSessionCommandError(command::Error),

    EnableSio32(bool),
    EnableSio32CommandError(command::Error),
}

impl Parsed {
    pub(in crate::engine) fn command(&self) -> engine::Command {
        match self {
            Self::BeginSession => engine::Command::BeginSession,
            Self::BeginSessionCommandError(_) => engine::Command::CommandError,
            Self::EnableSio32(true) => engine::Command::Sio32Mode,
            Self::EnableSio32(false) => engine::Command::Reset,
            Self::EnableSio32CommandError(_) => engine::Command::CommandError,
        }
    }

    pub(in crate::engine) fn revert(self) -> Command {
        match self {
            Self::BeginSession => Command::BeginSession,
            Self::BeginSessionCommandError(_) => Command::BeginSession,
            Self::EnableSio32(_) => Command::EnableSio32,
            Self::EnableSio32CommandError(_) => Command::EnableSio32,
        }
    }

    /// Send this data to wherever it needs to go and return what should be done next.
    pub(in crate::engine) fn finish(self) -> Finished {
        match self {
            Self::BeginSession => Finished::Success,
            Self::BeginSessionCommandError(error) => Finished::CommandError(error),
            Self::EnableSio32(true) => Finished::TransferLength(TransferLength::_32Bit),
            Self::EnableSio32(false) => Finished::TransferLength(TransferLength::_8Bit),
            Self::EnableSio32CommandError(error) => Finished::CommandError(error),
        }
    }
}
