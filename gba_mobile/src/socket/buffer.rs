use core::convert::Infallible;

pub trait Buffer {
    type ReadError: core::error::Error;
    type WriteError: core::error::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::ReadError>;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError>;

    fn len(&self) -> usize;
}

impl<const N: usize> Buffer for [u8; N] {
    type ReadError = Infallible;
    type WriteError = Infallible;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::ReadError> {
        Ok(0)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError> {
        Ok(0)
    }

    fn len(&self) -> usize {
        0
    }
}
