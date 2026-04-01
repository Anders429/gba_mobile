pub mod error;

mod pending;

pub use pending::Pending;

use crate::Generation;
use core::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub struct P2p;

#[derive(Clone, Copy, Debug)]
pub struct Socket1(pub(crate) Generation);

#[derive(Clone, Copy, Debug)]
pub struct Socket2(pub(crate) Generation);

#[derive(Debug)]
pub struct Connection<Driver, Socket> {
    link_generation: Generation,
    connection_generation: Generation,
    socket: Socket,
    driver: PhantomData<Driver>,
}
