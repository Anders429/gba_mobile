use crate::driver::{Command, sink};

/// A data source.
///
/// This is the source of data when sending a given picket.
pub(in crate::driver) trait Source: Copy {
    type Context: Default;

    fn command(self) -> Command;
    fn length(self, context: &Self::Context) -> u8;
    fn get(self, index: u8, context: &Self::Context) -> u8;
    fn sink(self) -> sink::Command;
}
