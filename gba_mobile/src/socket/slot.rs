use super::{Buffer, NoSocket, Socket, Status};
use crate::{
    config, dns,
    driver::active::{
        flow::{self, SocketSubFlow},
        queue::item::{self, ConnectionSubItem, SocketSubItem},
    },
};

pub(crate) trait Sealed: Sized {
    type ConnectionFlow: SocketSubFlow<Self>;
    type SocketFlow<const INDEX: usize>: SocketSubFlow<Self>;

    type ConnectionItem<Socket2, Dns, Config>: ConnectionSubItem<Self, Socket2, Dns, Config>
    where
        Socket2: Slot,
        Dns: dns::Mode,
        Config: config::Mode;
    type Socket1Item<Socket2, Dns, Config>: SocketSubItem<Self, Socket2, Dns, Config, 0>
    where
        Socket2: Slot,
        Dns: dns::Mode,
        Config: config::Mode;
    type Socket2Item<Socket1, Dns, Config>: SocketSubItem<Socket1, Self, Dns, Config, 1>
    where
        Socket1: Slot,
        Dns: dns::Mode,
        Config: config::Mode;

    fn ready_for_transfer(&mut self, trigger_frame: u8) -> bool;
}

impl<Buffer> Sealed for Socket<Buffer>
where
    Buffer: self::Buffer,
{
    type ConnectionFlow = flow::ConnectionFlow;
    type SocketFlow<const INDEX: usize> = flow::SocketFlow<INDEX>;

    type ConnectionItem<Socket2, Dns, Config>
        = item::Socket
    where
        Socket2: Slot,
        Dns: dns::Mode,
        Config: config::Mode;
    type Socket1Item<Socket2, Dns, Config>
        = item::Socket
    where
        Socket2: Slot,
        Dns: dns::Mode,
        Config: config::Mode;
    type Socket2Item<Socket1, Dns, Config>
        = item::Socket
    where
        Socket1: Slot,
        Dns: dns::Mode,
        Config: config::Mode;

    fn ready_for_transfer(&mut self, trigger_frame: u8) -> bool {
        if matches!(self.status, Status::Connected)
            && self.read_buffer.is_empty()
            && self.write_buffer.is_empty()
        {
            let result = self.frame == trigger_frame;
            self.frame = self.frame.saturating_add(1);
            result
        } else {
            false
        }
    }
}

impl Sealed for NoSocket {
    type ConnectionFlow = flow::Empty;
    type SocketFlow<const INDEX: usize> = flow::Empty;

    type ConnectionItem<Socket2, Dns, Config>
        = item::Empty
    where
        Socket2: Slot,
        Dns: dns::Mode,
        Config: config::Mode;
    type Socket1Item<Socket2, Dns, Config>
        = item::Empty
    where
        Socket2: Slot,
        Dns: dns::Mode,
        Config: config::Mode;
    type Socket2Item<Socket1, Dns, Config>
        = item::Empty
    where
        Socket1: Slot,
        Dns: dns::Mode,
        Config: config::Mode;

    fn ready_for_transfer(&mut self, _trigger_frame: u8) -> bool {
        false
    }
}

#[allow(private_bounds)]
pub trait Slot: Sealed {}

impl<Buffer> Slot for Socket<Buffer> where Buffer: self::Buffer {}

impl Slot for NoSocket {}
