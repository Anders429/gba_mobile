use core::convert::Infallible;

pub trait Buffer {
    type ReadError: core::error::Error;
    type WriteError: core::error::Error + Clone + 'static;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::ReadError>;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError>;

    fn is_empty(&self) -> bool;
}

impl<const N: usize> Buffer for [u8; N] {
    type ReadError = Infallible;
    type WriteError = Infallible;

    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::ReadError> {
        Ok(0)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::WriteError> {
        Ok(0)
    }

    fn is_empty(&self) -> bool {
        true
    }
}
