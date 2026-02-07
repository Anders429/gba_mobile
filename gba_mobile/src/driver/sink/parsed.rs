use super::{Command, Finished};
use crate::{
    driver::{self, command},
    mmio::serial::TransferLength,
};

#[derive(Debug)]
pub(in crate::driver) enum Parsed {
    BeginSession,
    BeginSessionCommandError(command::Error),

    EnableSio32(bool),
    EnableSio32CommandError(command::Error),
}

impl Parsed {
    pub(in crate::driver) fn command(&self) -> driver::Command {
        match self {
            Self::BeginSession => driver::Command::BeginSession,
            Self::BeginSessionCommandError(_) => driver::Command::CommandError,
            Self::EnableSio32(true) => driver::Command::Sio32Mode,
            Self::EnableSio32(false) => driver::Command::Reset,
            Self::EnableSio32CommandError(_) => driver::Command::CommandError,
        }
    }

    pub(in crate::driver) fn revert(self) -> Command {
        match self {
            Self::BeginSession => Command::BeginSession,
            Self::BeginSessionCommandError(_) => Command::BeginSession,
            Self::EnableSio32(_) => Command::EnableSio32,
            Self::EnableSio32CommandError(_) => Command::EnableSio32,
        }
    }

    /// Send this data to wherever it needs to go and return what should be done next.
    pub(in crate::driver) fn finish(self) -> Finished {
        match self {
            Self::BeginSession => Finished::Success,
            Self::BeginSessionCommandError(error) => Finished::CommandError(error),
            Self::EnableSio32(true) => Finished::TransferLength(TransferLength::_32Bit),
            Self::EnableSio32(false) => Finished::TransferLength(TransferLength::_8Bit),
            Self::EnableSio32CommandError(error) => Finished::CommandError(error),
        }
    }
}
