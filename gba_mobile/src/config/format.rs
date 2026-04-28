use deranged::RangedU8;

#[derive(Clone, Copy, Debug)]
pub struct Location {
    pub offset: u8,
    pub length: RangedU8<0, 128>,
}

pub trait Format: Sized + Clone {
    const WRITES: usize;

    type Segments: Segments<Format = Self>;
    type Error: Clone + core::error::Error + 'static;

    fn segments() -> Self::Segments;
    fn write(&self, request: usize, bytes: &mut [u8; 128]) -> Location;
}

pub enum ReadResult<Format, Segments> {
    Success(Format),
    Segments(Segments),
}

pub trait Segments: Sized {
    type Format: Format;

    fn location(&self) -> Location;
    fn read(
        self,
        bytes: &[u8],
    ) -> Result<ReadResult<Self::Format, Self>, <Self::Format as Format>::Error>;
}
