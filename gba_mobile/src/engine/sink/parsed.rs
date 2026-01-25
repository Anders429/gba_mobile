use super::{Command, Finished};
use crate::{engine, engine::command};

#[derive(Debug)]
pub(in crate::engine) enum Parsed {
    BeginSession,
    BeginSessionCommandError(command::Error),
}

impl Parsed {
    pub(in crate::engine) fn command(&self) -> engine::Command {
        match self {
            Self::BeginSession => engine::Command::BeginSession,
            Self::BeginSessionCommandError(_) => engine::Command::CommandError,
        }
    }

    pub(in crate::engine) fn revert(self) -> Command {
        match self {
            Self::BeginSession => Command::BeginSession,
            Self::BeginSessionCommandError(_) => Command::BeginSession,
        }
    }

    /// Send this data to wherever it needs to go and return what should be done next.
    pub(in crate::engine) fn finish(self) -> Finished {
        match self {
            Self::BeginSession => Finished::Success,
            Self::BeginSessionCommandError(error) => Finished::Success, // TODO: This needs to be something else I think.
        }
    }
}
