use crate::{ArrayVec, driver::Command};

#[derive(Debug)]
pub(in crate::driver::active) struct Data {
    pub(super) command: Command,
    pub(in crate::driver::active::flow) data: ArrayVec<u8, 255>,
}

impl Data {
    pub(in crate::driver::active) fn new() -> Self {
        Self {
            command: Command::Empty,
            data: ArrayVec::new(),
        }
    }
}
