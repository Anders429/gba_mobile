use super::{Buffer, NoSocket, Socket, Status};
use crate::driver::active::{
    flow::{self, SubFlowWithSocket},
    queue::item::{self, ConnectionSubItem, SocketSubItem},
};

pub(crate) trait Sealed: Sized {
    type ConnectionFlow: SubFlowWithSocket<Self>;
    type SocketFlow<const INDEX: usize>: SubFlowWithSocket<Self>;

    type ConnectionItem<Socket2>: ConnectionSubItem<Self, Socket2>
    where
        Socket2: Slot;
    type Socket1Item<Socket2>: SocketSubItem<Self, Socket2, 0>
    where
        Socket2: Slot;
    type Socket2Item<Socket1>: SocketSubItem<Socket1, Self, 1>
    where
        Socket1: Slot;

    fn ready_for_transfer(&mut self, trigger_frame: u8) -> bool;
}

impl<Buffer> Sealed for Socket<Buffer>
where
    Buffer: self::Buffer,
{
    type ConnectionFlow = flow::ConnectionFlow;
    type SocketFlow<const INDEX: usize> = flow::SocketFlow<INDEX>;

    type ConnectionItem<Socket2>
        = item::Socket
    where
        Socket2: Slot;
    type Socket1Item<Socket2>
        = item::Socket
    where
        Socket2: Slot;
    type Socket2Item<Socket1>
        = item::Socket
    where
        Socket1: Slot;

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

    type ConnectionItem<Socket2>
        = item::Empty
    where
        Socket2: Slot;
    type Socket1Item<Socket2>
        = item::Empty
    where
        Socket2: Slot;
    type Socket2Item<Socket1>
        = item::Empty
    where
        Socket1: Slot;

    fn ready_for_transfer(&mut self, _trigger_frame: u8) -> bool {
        false
    }
}

#[allow(private_bounds)]
pub trait Slot: Sealed {}

impl<Buffer> Slot for Socket<Buffer> where Buffer: self::Buffer {}

impl Slot for NoSocket {}
