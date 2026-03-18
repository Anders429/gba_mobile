mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;

use crate::Generation;

// TODO: All of this is very similar to P2P connections. Consider combining with a generic.

#[derive(Debug)]
pub struct PPP {
    link_generation: Generation,
    connection_generation: Generation,
}
