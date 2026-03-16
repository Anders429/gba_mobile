use super::ReadConfig;
use core::{
    fmt,
    fmt::{Display, Formatter},
};
use deranged::RangedU8;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) enum Data {
    Offset,
    Config {
        index: RangedU8<0, 127>,
        data: [u8; 128],
    },
}

impl Data {
    pub(super) fn new() -> Self {
        Self::Offset
    }

    fn new_config() -> Self {
        Self::Config {
            index: RangedU8::new_static::<0>(),
            data: [0; 128],
        }
    }

    pub(super) fn receive_data(
        self,
        byte: u8,
        read_config: ReadConfig,
    ) -> Result<Either<Self, [u8; 128]>, (Error, Option<u16>)> {
        match self {
            Self::Offset => match (read_config, byte) {
                (ReadConfig::FirstHalf, 0) => Ok(Either::Left(Self::new_config())),
                (ReadConfig::FirstHalf, _) => Err((Error::FirstHalfOffset(byte), Some(1))),
                (ReadConfig::SecondHalf, 128) => Ok(Either::Left(Self::new_config())),
                (ReadConfig::SecondHalf, _) => Err((Error::SecondHalfOffset(byte), Some(1))),
            },
            Self::Config { index, mut data } => {
                data[index.get() as usize] = byte;
                match index.checked_add(1) {
                    Some(index) => Ok(Either::Left(Self::Config { index, data })),
                    None => Ok(Either::Right(data)),
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    FirstHalfOffset(u8),
    SecondHalfOffset(u8),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::FirstHalfOffset(offset) => write!(
                formatter,
                "received offset of {offset} when reading first half of config, but expected offset of 0"
            ),
            Self::SecondHalfOffset(offset) => write!(
                formatter,
                "received offset of {offset} when reading second half of config, but expected offset of 128"
            ),
        }
    }
}

impl core::error::Error for Error {}
