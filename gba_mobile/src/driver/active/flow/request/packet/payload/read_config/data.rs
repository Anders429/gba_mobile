use crate::config::format::Location;

use core::{
    fmt,
    fmt::{Display, Formatter},
};
use either::Either;

#[derive(Debug)]
pub(in crate::driver) enum Data {
    Offset,
    Config { index: u8, data: [u8; 128] },
}

impl Data {
    pub(super) fn new() -> Self {
        Self::Offset
    }

    fn new_config() -> Self {
        Self::Config {
            index: 0,
            data: [0; 128],
        }
    }

    pub(super) fn receive_data(
        self,
        byte: u8,
        location: Location,
    ) -> Result<Either<Self, [u8; 128]>, (Error, Option<u16>)> {
        match self {
            Self::Offset => {
                if byte == location.offset {
                    if location.length.get() == 0 {
                        Ok(Either::Right([0; 128]))
                    } else {
                        Ok(Either::Left(Self::new_config()))
                    }
                } else {
                    Err((Error::Offset(byte, location.offset), Some(1)))
                }
            }
            Self::Config { index, mut data } => {
                data[index as usize] = byte;
                if index + 1 == location.length.get() {
                    Ok(Either::Right(data))
                } else {
                    Ok(Either::Left(Self::Config {
                        index: index + 1,
                        data,
                    }))
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Offset(u8, u8),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Offset(received, expected) => write!(
                formatter,
                "received offset of {received} when reading config, but expected offset of {expected}",
            ),
        }
    }
}

impl core::error::Error for Error {}
