use super::{Buffer, NoSocket, Socket, Status};
use crate::{
    dns,
    driver::active::{
        flow::{self, SocketSubFlow},
        queue::item::{self, ConnectionSubItem, SocketSubItem},
    },
};

pub(crate) trait Sealed: Sized {
    type ConnectionFlow: SocketSubFlow<Self>;
    type SocketFlow<const INDEX: usize>: SocketSubFlow<Self>;

    type ConnectionItem<Socket2, Dns>: ConnectionSubItem<Self, Socket2, Dns>
    where
        Socket2: Slot,
        Dns: dns::Mode;
    type Socket1Item<Socket2, Dns>: SocketSubItem<Self, Socket2, Dns, 0>
    where
        Socket2: Slot,
        Dns: dns::Mode;
    type Socket2Item<Socket1, Dns>: SocketSubItem<Socket1, Self, Dns, 1>
    where
        Socket1: Slot,
        Dns: dns::Mode;

    fn ready_for_transfer(&mut self, trigger_frame: u8) -> bool;
}

impl<Buffer> Sealed for Socket<Buffer>
where
    Buffer: self::Buffer,
{
    type ConnectionFlow = flow::ConnectionFlow;
    type SocketFlow<const INDEX: usize> = flow::SocketFlow<INDEX>;

    type ConnectionItem<Socket2, Dns>
        = item::Socket
    where
        Socket2: Slot,
        Dns: dns::Mode;
    type Socket1Item<Socket2, Dns>
        = item::Socket
    where
        Socket2: Slot,
        Dns: dns::Mode;
    type Socket2Item<Socket1, Dns>
        = item::Socket
    where
        Socket1: Slot,
        Dns: dns::Mode;

    fn ready_for_transfer(&mut self, trigger_frame: u8) -> bool {
        if matches!(self.status, Status::Connected) && self.read_buffer.is_empty() {
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

    type ConnectionItem<Socket2, Dns>
        = item::Empty
    where
        Socket2: Slot,
        Dns: dns::Mode;
    type Socket1Item<Socket2, Dns>
        = item::Empty
    where
        Socket2: Slot,
        Dns: dns::Mode;
    type Socket2Item<Socket1, Dns>
        = item::Empty
    where
        Socket1: Slot,
        Dns: dns::Mode;

    fn ready_for_transfer(&mut self, _trigger_frame: u8) -> bool {
        false
    }
}

#[allow(private_bounds)]
pub trait Slot: Sealed {}

impl<Buffer> Slot for Socket<Buffer> where Buffer: self::Buffer {}

impl Slot for NoSocket {}
