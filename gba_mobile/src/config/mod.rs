pub mod mobile_system_gb;

pub trait Config: Sized {
    type Error: core::error::Error + 'static;

    fn read(bytes: &[u8; 256]) -> Result<Self, Self::Error>;
    fn write(&self, bytes: &mut [u8; 256]);
}
