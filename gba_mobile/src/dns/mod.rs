mod error;
mod pending;
mod to_name;

pub use error::Error;
pub use to_name::ToName;

pub(crate) use pending::Pending;

use crate::{
    ArrayVec, Generation,
    driver::active::{
        flow::{self, DnsSubFlow},
        queue::item::{self, DnsSubItem},
    },
    socket,
};
use core::net::Ipv4Addr;

pub(crate) trait Sealed: Sized {
    type Item<Socket1, Socket2>: DnsSubItem<Socket1, Socket2, Self>
    where
        Socket1: socket::slot::Sealed,
        Socket2: socket::slot::Sealed;
    type Flow: DnsSubFlow<Self>;
}

#[allow(private_bounds)]
pub trait Mode: Sealed {}

#[derive(Debug)]
pub struct NoDns;

impl Sealed for NoDns {
    type Item<Socket1, Socket2>
        = item::Empty
    where
        Socket1: socket::slot::Sealed,
        Socket2: socket::slot::Sealed;
    type Flow = flow::Empty;
}

impl Mode for NoDns {}

#[derive(Debug)]
pub(crate) enum State<const MAX_LEN: usize> {
    Request(ArrayVec<u8, MAX_LEN>),
    Success(Ipv4Addr),
    NotFound,
}

#[derive(Debug)]
pub struct Dns<const MAX_LEN: usize> {
    pub(crate) state: State<MAX_LEN>,
    pub(crate) generation: Generation,
}

impl<const MAX_LEN: usize> Dns<MAX_LEN> {
    pub const fn new() -> Self {
        Self {
            state: State::Request(ArrayVec::new()),
            generation: Generation::new(),
        }
    }
}

impl<const MAX_LEN: usize> Sealed for Dns<MAX_LEN> {
    type Item<Socket1, Socket2>
        = item::Dns
    where
        Socket1: socket::slot::Sealed,
        Socket2: socket::slot::Sealed;
    type Flow = flow::DnsFlow<MAX_LEN>;
}

impl<const MAX_LEN: usize> Mode for Dns<MAX_LEN> {}
