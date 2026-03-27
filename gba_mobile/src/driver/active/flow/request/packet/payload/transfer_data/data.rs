use core::num::NonZeroU16;

use super::error;
use crate::{ArrayVec, driver::active::socket};
use deranged::RangedU8;
use either::Either;

#[derive(Debug)]
enum State {
    Id(socket::Id),
    Data(ArrayVec<u8, 254>),
}

#[derive(Debug)]
pub(super) struct Data {
    state: State,
    length: RangedU8<0, 254>,
}

impl Data {
    pub(super) fn new(id: socket::Id, length: RangedU8<0, 254>) -> Self {
        Self {
            state: State::Id(id),
            length,
        }
    }

    pub(super) fn receive_data(
        self,
        byte: u8,
    ) -> Result<Either<Self, ArrayVec<u8, 254>>, (error::InvalidData, Option<(NonZeroU16, u16)>)>
    {
        match self.state {
            State::Id(expected) => {
                let received = socket::Id::from(byte);
                if expected == received {
                    if self.length.get() == 0 {
                        Ok(Either::Right(ArrayVec::new()))
                    } else {
                        Ok(Either::Left(Self {
                            state: State::Data(ArrayVec::new()),
                            length: self.length,
                        }))
                    }
                } else {
                    if self.length.get() == 0 {
                        Err((
                            error::InvalidData::IncorrectSocketId { received, expected },
                            None,
                        ))
                    } else {
                        Err((
                            error::InvalidData::IncorrectSocketId { received, expected },
                            Some((
                                unsafe { NonZeroU16::new_unchecked(self.length.get() as u16 + 1) },
                                1,
                            )),
                        ))
                    }
                }
            }
            State::Data(mut data) => {
                // SAFETY: This will always succeed, because we are guaranteed to exit if the
                // ArrayVec's capacity is filled.
                unsafe {
                    data.try_push(byte).unwrap_unchecked();
                }
                if data.len() >= self.length.get() {
                    Ok(Either::Right(data))
                } else {
                    Ok(Either::Left(Self {
                        state: State::Data(data),
                        length: self.length,
                    }))
                }
            }
        }
    }
}
