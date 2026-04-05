pub(crate) mod item;

use super::{ConnectionRequest, Flow, Phase, State};
use crate::{Timer, dns, socket};
use core::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    ops::BitOr,
};
use item::{ConnectionSubItem, DnsSubItem, Item, SocketSubItem};

pub(super) struct Queue<Socket1, Socket2, Dns> {
    bits: u16,
    sockets: PhantomData<(Socket1, Socket2)>,
    dns: PhantomData<Dns>,
}

impl<Socket1, Socket2, Dns> Queue<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    const NONE: Self = Self::bits(0b0000_0000_0000_0000);

    const START: Self = Self::bits(0b0000_0000_0000_0001);
    const END: Self = Self::bits(0b0000_0000_0000_0010);

    /// Will connect based on the connection parameters.
    ///
    /// These indicate whether this is a connect, accept, or login flow.
    const CONNECT: Self = Self::bits(0b0000_0000_0000_0100);
    const DISCONNECT: Self = Self::bits(0b0000_0000_0000_1000);

    /// Based on the socket, we will either do TCP or UDP here.
    ///
    /// Additionally, we'll include DNS or not based on whether we have a URL or an IP.
    const SOCKET_1_OPEN: Self = Self::bits(0b0000_0000_0001_0000);
    const SOCKET_1_CLOSE: Self = Self::bits(0b0000_0000_0010_0000);
    const SOCKET_2_OPEN: Self = Self::bits(0b0000_0000_0100_0000);
    const SOCKET_2_CLOSE: Self = Self::bits(0b0000_0000_1000_0000);

    /// These are used whether we are connected to the internet or not.
    ///
    /// If we attempt to use a socket that is currently not configured, the data read will be
    /// dropped.
    const SOCKET_1_TRANSFER: Self = Self::bits(0b0000_0001_0000_0000);
    const SOCKET_2_TRANSFER: Self = Self::bits(0b0000_0010_0000_0000);

    /// To ensure we read/write on both sockets equally, we use this bit to toggle which should be
    /// read/written with higher priority. This way, if we continually write to both, we will see
    /// progress made on the communication over both as well.
    const SOCKET_2_PRIORITY: Self = Self::bits(0b0000_0100_0000_0000);

    const DNS: Self = Self::bits(0b0001_0000_0000_0000);

    const WRITE_CONFIG: Self = Self::bits(0b0010_0000_0000_0000);

    const STATUS: Self = Self::bits(0b0100_0000_0000_0000);
    const IDLE: Self = Self::bits(0b1000_0000_0000_0000);

    const fn bits(bits: u16) -> Self {
        Self {
            bits,
            sockets: PhantomData,
            dns: PhantomData,
        }
    }

    pub(super) fn new() -> Self {
        Self::NONE
    }

    pub(super) fn set_start(&mut self) {
        self.set(Self::START);
    }

    pub(super) fn set_end(&mut self) {
        if self.has(Self::START) {
            // If we have a start already scheduled, we just cancel it out.
            self.clear(Self::START);
        } else {
            // Otherwise, we end the session we are currently in.
            self.set(Self::END);
        }
    }

    pub(super) fn set_connect(&mut self) {
        self.set(Self::CONNECT);
    }

    pub(super) fn set_disconnect(&mut self) {
        if self.has(Self::CONNECT) {
            self.clear(Self::CONNECT);
        } else {
            self.set(Self::DISCONNECT);
        }
    }

    pub(super) fn set_socket_1_open(&mut self) {
        self.set(Self::SOCKET_1_OPEN);
    }

    pub(super) fn set_socket_1_close(&mut self) {
        if self.has(Self::SOCKET_1_OPEN) {
            self.clear(Self::SOCKET_1_OPEN);
        } else {
            self.set(Self::SOCKET_1_CLOSE);
        }
    }

    pub(super) fn set_socket_2_open(&mut self) {
        self.set(Self::SOCKET_2_OPEN);
    }

    pub(super) fn set_socket_2_close(&mut self) {
        if self.has(Self::SOCKET_2_OPEN) {
            self.clear(Self::SOCKET_2_OPEN);
        } else {
            self.set(Self::SOCKET_2_CLOSE);
        }
    }

    pub(super) fn set_socket_1_transfer(&mut self) {
        self.set(Self::SOCKET_1_TRANSFER);
    }

    pub(super) fn set_socket_2_transfer(&mut self) {
        self.set(Self::SOCKET_2_TRANSFER);
    }

    pub(super) fn set_dns(&mut self) {
        self.set(Self::DNS);
    }

    pub(super) fn set_write_config(&mut self) {
        self.set(Self::WRITE_CONFIG);
    }

    pub(super) fn set_status(&mut self) {
        self.set(Self::STATUS);
    }

    pub(super) fn set_idle(&mut self) {
        self.set(Self::IDLE);
    }

    pub(super) fn next_flow(
        &mut self,
        state: &mut State,
        timer: Timer,
        socket_1: &mut Socket1,
        socket_2: &mut Socket2,
        dns: &Dns,
    ) -> Option<Flow<Socket1, Socket2, Dns>> {
        self.next().and_then(|item| {
            match item {
                Item::Start => Some(Flow::start(state.transfer_length)),
                Item::End => Some(Flow::end(state.transfer_length, timer)),
                Item::Reset => Some(Flow::reset(state.transfer_length, timer)),
                Item::Disconnect => Some(Flow::disconnect(state.transfer_length, timer)),
                Item::Connect => match &state.phase {
                    Phase::Connecting(ConnectionRequest::Accept { .. }) => {
                        Socket1::ConnectionItem::accept(state, timer)
                    }
                    Phase::Connecting(ConnectionRequest::Connect { phone_number }) => {
                        Socket1::ConnectionItem::connect(phone_number, state, timer)
                    }
                    Phase::Connecting(ConnectionRequest::Login {
                        phone_number,
                        id,
                        password,
                        primary_dns,
                        secondary_dns,
                    }) => Some(Flow::login(
                        state.transfer_length,
                        timer,
                        state.adapter,
                        phone_number.clone(),
                        id.clone(),
                        password.clone(),
                        *primary_dns,
                        *secondary_dns,
                        state.connection_generation,
                    )),
                    // If we have this item on the queue, but have left the connecting phase, we
                    // can't determine what type of connection request should be attempted. This
                    // also means we likely have ended the session anyway, so a connection attempt
                    // would be pointless.
                    _ => None,
                },
                Item::Socket1(item) => item.next_flow(state, timer, socket_1, socket_2),
                Item::Socket2(item) => item.next_flow(state, timer, socket_1, socket_2),
                Item::Dns(item) => item.flow(dns, state, timer),
                Item::WriteConfig => Some(Flow::write_config(
                    state.transfer_length,
                    timer,
                    &state.config,
                )),
                Item::Status => Some(Flow::status(state.transfer_length, timer)),
                Item::Idle => Some(Flow::idle(state.transfer_length, timer)),
            }
        })
    }

    fn has(&self, bits: Self) -> bool {
        self.bits & bits.bits == bits.bits
    }

    fn set(&mut self, bits: Self) {
        self.bits = self.bits | bits.bits
    }

    fn clear(&mut self, bits: Self) {
        self.bits = self.bits & !bits.bits
    }

    fn clear_socket_1_transfer(&mut self) {
        self.set(Queue::SOCKET_2_PRIORITY);
        self.clear(Queue::SOCKET_1_TRANSFER);
    }

    fn clear_socket_2_transfer(&mut self) {
        self.clear(Queue::SOCKET_2_PRIORITY | Queue::SOCKET_2_TRANSFER);
    }

    fn clear_session(&mut self) {
        self.clear(
            Queue::START
                | Queue::END
                | Queue::DISCONNECT
                | Queue::SOCKET_1_CLOSE
                | Queue::SOCKET_2_CLOSE
                | Queue::STATUS
                | Queue::IDLE,
        );
    }

    fn clear_disconnect(&mut self) {
        self.clear(
            Queue::DISCONNECT | Queue::SOCKET_1_CLOSE | Queue::SOCKET_2_CLOSE | Queue::STATUS,
        );
    }

    fn clear_connect(&mut self) {
        self.clear(Queue::CONNECT | Queue::SOCKET_1_CLOSE | Queue::SOCKET_2_CLOSE | Queue::STATUS);
    }
}

impl<Socket1, Socket2, Dns> BitOr for Queue<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        Self::bits(self.bits | other.bits)
    }
}

impl<Socket1, Socket2, Dns> Clone for Queue<Socket1, Socket2, Dns> {
    fn clone(&self) -> Self {
        Self {
            bits: self.bits,
            sockets: self.sockets,
            dns: self.dns,
        }
    }
}

impl<Socket1, Socket2, Dns> Debug for Queue<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_list().entries(self.clone()).finish()
    }
}

impl<Socket1, Socket2, Dns> Iterator for Queue<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    type Item = Item<Socket1, Socket2, Dns>;

    /// Iterates over the items set on the queue, returning highest priority items first.
    ///
    /// Redundant items will be queued as necessary. For example, if we are ending the session, the
    /// status bit is redundant as it will make no difference afterward.
    fn next(&mut self) -> Option<Self::Item> {
        if self.has(Queue::SOCKET_2_PRIORITY | Queue::SOCKET_2_TRANSFER) {
            self.clear_socket_2_transfer();
            Some(Item::Socket2(Socket2::Socket2Item::transfer()))
        } else if self.has(Queue::SOCKET_1_TRANSFER) {
            self.clear_socket_1_transfer();
            Some(Item::Socket1(Socket1::Socket1Item::transfer()))
        } else if self.has(Queue::SOCKET_2_TRANSFER) {
            self.clear_socket_2_transfer();
            Some(Item::Socket2(Socket2::Socket2Item::transfer()))
        } else if self.has(Queue::WRITE_CONFIG) {
            self.clear(Queue::WRITE_CONFIG);
            Some(Item::WriteConfig)
        } else if self.has(Queue::DNS) {
            self.clear(Queue::DNS);
            Some(Item::Dns(Dns::Item::dns()))
        } else if self.has(Queue::START | Queue::END) {
            // When both start and end are set, we combine them into a single reset flow.
            self.clear_session();
            Some(Item::Reset)
        } else if self.has(Queue::END) {
            self.clear_session();
            Some(Item::End)
        } else if self.has(Queue::START) {
            self.clear_session();
            Some(Item::Start)
        } else if self.has(Queue::DISCONNECT) {
            self.clear_disconnect();
            Some(Item::Disconnect)
        } else if self.has(Queue::CONNECT) {
            self.clear_connect();
            Some(Item::Connect)
        } else if self.has(Queue::SOCKET_1_CLOSE) {
            self.clear(Queue::SOCKET_1_CLOSE);
            Some(Item::Socket1(Socket1::Socket1Item::close()))
        } else if self.has(Queue::SOCKET_2_CLOSE) {
            self.clear(Queue::SOCKET_2_CLOSE);
            Some(Item::Socket2(Socket2::Socket2Item::close()))
        } else if self.has(Queue::SOCKET_1_OPEN) {
            self.clear(Queue::SOCKET_1_OPEN);
            Some(Item::Socket1(Socket1::Socket1Item::open()))
        } else if self.has(Queue::SOCKET_2_OPEN) {
            self.clear(Queue::SOCKET_2_OPEN);
            Some(Item::Socket2(Socket2::Socket2Item::open()))
        } else if self.has(Queue::STATUS) {
            self.clear(Queue::STATUS);
            Some(Item::Status)
        } else if self.has(Queue::IDLE) {
            self.clear(Queue::IDLE);
            Some(Item::Idle)
        } else {
            None
        }
    }
}
