pub mod mobile_system_gb;

use crate::{
    dns,
    driver::active::{
        flow::{self, ConfigFlow, ConfigSubFlow},
        queue::item::{self, ConfigSubItem},
    },
    socket,
};
use core::mem::MaybeUninit;

pub trait Format: Sized + Clone {
    type Error: Clone + core::error::Error + 'static;

    fn read(bytes: &[u8; 256]) -> Result<Self, Self::Error>;
    fn write(&self, bytes: &mut [u8; 256]);
}

pub(crate) trait Sealed: Sized {
    type Item<Socket1, Socket2, Dns>: ConfigSubItem<Socket1, Socket2, Dns, Self>
    where
        Socket1: socket::Slot,
        Socket2: socket::Slot,
        Dns: dns::Sealed;
    type Flow: ConfigSubFlow<Self>;
}

#[allow(private_bounds)]
pub trait Mode: Sealed {}

#[derive(Debug)]
pub struct NoConfig;

impl Sealed for NoConfig {
    type Item<Socket1, Socket2, Dns>
        = item::Empty
    where
        Socket1: socket::Slot,
        Socket2: socket::Slot,
        Dns: dns::Sealed;
    type Flow = flow::Empty;
}

impl Mode for NoConfig {}

#[derive(Debug)]
pub struct Config<Format>
where
    Format: self::Format,
{
    pub(crate) data: MaybeUninit<Result<Format, Format::Error>>,
}

impl<Format> Config<Format>
where
    Format: self::Format,
{
    pub const fn new() -> Self {
        Self {
            // This value will be initialized upon linking with the adapter.
            data: MaybeUninit::uninit(),
        }
    }
}

impl<Format> Sealed for Config<Format>
where
    Format: self::Format,
{
    type Item<Socket1, Socket2, Dns>
        = item::WriteConfig
    where
        Socket1: socket::Slot,
        Socket2: socket::Slot,
        Dns: dns::Sealed;
    type Flow = ConfigFlow;
}

impl<Format> Mode for Config<Format> where Format: self::Format {}
