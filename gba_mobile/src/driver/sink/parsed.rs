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

    WaitForCall,
    WaitForCallCommandError(command::Error),

    Call,
    CallCommandError(command::Error),

    Reset,
    ResetCommandError(command::Error),

    EndSession,
    EndSessionCommandError(command::Error),
}

impl Parsed {
    pub(in crate::driver) fn command(&self) -> driver::Command {
        match self {
            Self::BeginSession => driver::Command::BeginSession,
            Self::BeginSessionCommandError(_) => driver::Command::CommandError,
            Self::EnableSio32(true) => driver::Command::Sio32Mode,
            Self::EnableSio32(false) => driver::Command::Reset,
            Self::EnableSio32CommandError(_) => driver::Command::CommandError,

            Self::WaitForCall => driver::Command::WaitForTelephoneCall,
            Self::WaitForCallCommandError(_) => driver::Command::CommandError,
            Self::Call => driver::Command::DialTelephone,
            Self::CallCommandError(_) => driver::Command::CommandError,

            Self::Reset => driver::Command::Reset,
            Self::ResetCommandError(_) => driver::Command::CommandError,
            Self::EndSession => driver::Command::EndSession,
            Self::EndSessionCommandError(_) => driver::Command::CommandError,
        }
    }

    pub(in crate::driver) fn revert(self) -> Command {
        match self {
            Self::BeginSession => Command::BeginSession,
            Self::BeginSessionCommandError(_) => Command::BeginSession,
            Self::EnableSio32(_) => Command::EnableSio32,
            Self::EnableSio32CommandError(_) => Command::EnableSio32,

            Self::WaitForCall => Command::WaitForCall,
            Self::WaitForCallCommandError(_) => Command::WaitForCall,
            Self::Call => Command::Call,
            Self::CallCommandError(_) => Command::Call,

            Self::Reset => Command::Reset,
            Self::ResetCommandError(_) => Command::Reset,
            Self::EndSession => Command::EndSession,
            Self::EndSessionCommandError(_) => Command::EndSession,
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

            Self::WaitForCall => Finished::Success,
            Self::WaitForCallCommandError(error) => Finished::CommandError(error),
            Self::Call => Finished::Success,
            Self::CallCommandError(error) => Finished::CommandError(error),

            // Ending or resetting the session will reset transfer length back to 8-bit mode.
            Self::Reset => Finished::TransferLength(TransferLength::_8Bit),
            Self::ResetCommandError(error) => Finished::CommandError(error),
            Self::EndSession => Finished::TransferLength(TransferLength::_8Bit),
            Self::EndSessionCommandError(error) => Finished::CommandError(error),
        }
    }
}
