use crate::{
    arrayvec::ArrayVec,
    driver,
    driver::{Adapter, Command, sink},
    phone_number::Digit,
};

#[derive(Debug, Default)]
pub(in crate::driver) struct Context {
    pub(in crate::driver) phone_number: ArrayVec<Digit, 32>,
    pub(in crate::driver) adapter: Adapter,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::driver) enum Source {
    Call,
}

impl driver::Source for Source {
    type Context = Context;

    fn command(self) -> Command {
        match self {
            Self::Call => Command::DialTelephone,
        }
    }

    fn length(self, context: &Self::Context) -> u8 {
        match self {
            Self::Call => context.phone_number.len() + 1,
        }
    }

    fn get(self, index: u8, context: &Self::Context) -> u8 {
        match self {
            Self::Call => {
                if index == 0 {
                    context.adapter.dial_byte()
                } else {
                    context
                        .phone_number
                        .get(index - 1)
                        .map(|&digit| digit.into())
                        .unwrap_or(0x00)
                }
            }
        }
    }

    fn sink(self) -> sink::Command {
        match self {
            Self::Call => sink::Command::Call,
        }
    }
}
