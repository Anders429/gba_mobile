pub mod format;
pub mod mobile_system_gb;

pub use format::Format;

use crate::{
    dns,
    driver::active::{
        flow::{self, ConfigFlow, ConfigSubFlow},
        queue::item::{self, ConfigSubItem},
    },
    socket,
};
use core::{
    fmt,
    fmt::{Debug, Formatter},
};

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

pub(crate) enum Data<Format>
where
    Format: self::Format,
{
    Segments(Format::Segments),
    Config(Format),
    Error(Format::Error),
}

impl<Format> Debug for Data<Format>
where
    Format: self::Format + Debug,
    Format::Segments: Debug,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Segments(segments) => formatter.debug_tuple("Segments").field(segments).finish(),
            Self::Config(format) => formatter.debug_tuple("Config").field(format).finish(),
            Self::Error(error) => formatter.debug_tuple("Error").field(error).finish(),
        }
    }
}

pub struct Config<Format>
where
    Format: self::Format,
{
    pub(crate) data: Data<Format>,
}

impl<Format> Config<Format>
where
    Format: self::Format,
{
    pub const fn new(segments: Format::Segments) -> Self {
        Self {
            // This value will be initialized upon linking with the adapter.
            data: Data::Segments(segments),
        }
    }
}

impl<Format> Debug for Config<Format>
where
    Format: self::Format + Debug,
    Format::Segments: Debug,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("Config")
            .field("data", &self.data)
            .finish()
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
    type Flow = ConfigFlow<Format>;
}

impl<Format> Mode for Config<Format> where Format: self::Format {}
