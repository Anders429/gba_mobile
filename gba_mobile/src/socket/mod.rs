pub mod to_socket;

mod error;
mod pending;

pub use error::Error;
pub use pending::Pending;
pub use to_socket::ToSocket;

use crate::Generation;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Index {
    One,
    Two,
}

impl From<Index> for usize {
    fn from(index: Index) -> Self {
        match index {
            Index::One => 0,
            Index::Two => 1,
        }
    }
}

#[derive(Debug)]
pub struct Socket {
    link_generation: Generation,
    connection_generation: Generation,
    socket_generation: Generation,
    index: Index,
}
