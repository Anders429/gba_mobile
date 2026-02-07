mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::Generation;

#[derive(Debug)]
pub struct LinkP2P {
    generation: Generation,
}
