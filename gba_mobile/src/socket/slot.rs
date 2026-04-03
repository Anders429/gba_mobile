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

    fn vblank_info<'a>(&'a mut self) -> Option<(&'a mut u8, &'a Status)>;
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

    fn vblank_info<'a>(&'a mut self) -> Option<(&'a mut u8, &'a Status)> {
        Some((&mut self.frame, &self.status))
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

    fn vblank_info<'a>(&'a mut self) -> Option<(&'a mut u8, &'a Status)> {
        None
    }
}

#[allow(private_bounds)]
pub trait Slot: Sealed {}

impl<Buffer> Slot for Socket<Buffer> where Buffer: self::Buffer {}

impl Slot for NoSocket {}
