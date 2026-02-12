mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::Generation;

#[derive(Debug)]
pub struct P2P {
    generation: Generation,
    call_generation: Generation,
}
