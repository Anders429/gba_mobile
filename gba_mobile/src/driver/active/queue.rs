use super::{ConnectionRequest, Flow, Phase, State};
use core::{
    fmt::{self, Debug, Formatter},
    ops::BitOr,
};

#[derive(Clone)]
pub(super) struct Queue(u16);

impl Queue {
    const NONE: Self = Self(0b0000_0000_0000_0000);

    const START: Self = Self(0b0000_0000_0000_0001);
    const END: Self = Self(0b0000_0000_0000_0010);

    /// Will connect based on the connection parameters.
    ///
    /// These indicate whether this is a connect, accept, or login flow.
    const CONNECT: Self = Self(0b0000_0000_0000_0100);
    const DISCONNECT: Self = Self(0b0000_0000_0000_1000);

    /// Based on the socket, we will either do TCP or UDP here.
    ///
    /// Additionally, we'll include DNS or not based on whether we have a URL or an IP.
    const SOCKET_1_OPEN: Self = Self(0b0000_0000_0001_0000);
    const SOCKET_1_CLOSE: Self = Self(0b0000_0000_0010_0000);
    const SOCKET_2_OPEN: Self = Self(0b0000_0000_0100_0000);
    const SOCKET_2_CLOSE: Self = Self(0b0000_0000_1000_0000);

    /// These are used whether we are connected to the internet or not.
    ///
    /// If we attempt to use a socket that is currently not configured, the data read will be
    /// dropped.
    const SOCKET_1_READ: Self = Self(0b0000_0001_0000_0000);
    const SOCKET_1_WRITE: Self = Self(0b0000_0010_0000_0000);
    const SOCKET_2_READ: Self = Self(0b0000_00100_0000_0000);
    const SOCKET_2_WRITE: Self = Self(0b0000_1000_0000_0000);
    // TODO: Can read and write be combined?

    /// To ensure we read/write on both sockets equally, we use this bit to toggle which should be
    /// read/written with higher priority. This way, if we continually write to both, we will see
    /// progress made on the communication over both as well.
    const SOCKET_2_PRIORITY: Self = Self(0b0001_0000_0000_0000);

    const WRITE_CONFIG: Self = Self(0b0010_0000_0000_0000);

    const STATUS: Self = Self(0b0100_0000_0000_0000);
    const IDLE: Self = Self(0b1000_0000_0000_0000);
}

impl Queue {
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

    pub(super) fn set_socket_1_read(&mut self) {
        self.set(Self::SOCKET_1_READ);
    }

    pub(super) fn set_socket_1_write(&mut self) {
        self.set(Self::SOCKET_1_WRITE);
    }

    pub(super) fn set_socket_2_read(&mut self) {
        self.set(Self::SOCKET_2_READ);
    }

    pub(super) fn set_socket_2_write(&mut self) {
        self.set(Self::SOCKET_2_WRITE);
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

    pub(super) fn next_flow(&mut self, state: &State) -> Option<Flow> {
        self.next().and_then(|item| {
            match item {
                Item::Start => Some(Flow::start(state.transfer_length)),
                Item::End => Some(Flow::end(state.transfer_length, state.timer)),
                Item::Reset => Some(Flow::reset(state.transfer_length, state.timer)),
                Item::Connect => match &state.phase {
                    Phase::Connecting(ConnectionRequest::Accept { .. }) => {
                        Some(Flow::accept(state.transfer_length, state.timer))
                    }
                    Phase::Connecting(ConnectionRequest::Connect { phone_number }) => {
                        Some(Flow::connect(
                            state.transfer_length,
                            state.timer,
                            state.adapter,
                            phone_number.clone(),
                            state.connection_generation,
                        ))
                    }
                    Phase::Connecting(_) => todo!(),
                    // If we have this item on the queue, but have left the connecting phase, we
                    // can't determine what type of connection request should be attempted. This
                    // also means we likely have ended the session anyway, so a connection attempt
                    // would be pointless.
                    _ => None,
                },
                Item::WriteConfig => Some(Flow::write_config(
                    state.transfer_length,
                    state.timer,
                    &state.config,
                )),
                Item::Idle => Some(Flow::idle(state.transfer_length, state.timer)),
                _ => todo!(),
            }
        })
    }

    fn has(&self, bits: Self) -> bool {
        self.0 & bits.0 == bits.0
    }

    fn set(&mut self, bits: Self) {
        self.0 = self.0 | bits.0
    }

    fn clear(&mut self, bits: Self) {
        self.0 = self.0 & !bits.0
    }

    fn clear_socket_1_transfer_data(&mut self) {
        self.set(Queue::SOCKET_2_PRIORITY);
        self.clear(Queue::SOCKET_1_WRITE | Queue::SOCKET_1_READ);
    }

    fn clear_socket_2_transfer_data(&mut self) {
        self.clear(Queue::SOCKET_2_PRIORITY | Queue::SOCKET_2_WRITE | Queue::SOCKET_2_READ);
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

impl BitOr for Queue {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        Self(self.0 | other.0)
    }
}

impl Debug for Queue {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_list().entries(self.clone()).finish()
    }
}

impl Iterator for Queue {
    type Item = Item;

    /// Iterates over the items set on the queue, returning highest priority items first.
    ///
    /// Redundant items will be queued as necessary. For example, if we are ending the session, the
    /// status bit is redundant as it will make no difference afterward.
    fn next(&mut self) -> Option<Self::Item> {
        if self.has(Queue::SOCKET_2_PRIORITY | Queue::SOCKET_2_WRITE) {
            self.clear_socket_2_transfer_data();
            Some(Item::Socket2Write)
        } else if self.has(Queue::SOCKET_2_PRIORITY | Queue::SOCKET_2_READ) {
            self.clear_socket_2_transfer_data();
            Some(Item::Socket2Read)
        } else if self.has(Queue::SOCKET_1_WRITE) {
            self.clear_socket_1_transfer_data();
            Some(Item::Socket1Write)
        } else if self.has(Queue::SOCKET_1_READ) {
            self.clear_socket_1_transfer_data();
            Some(Item::Socket1Read)
        } else if self.has(Queue::SOCKET_2_WRITE) {
            self.clear_socket_2_transfer_data();
            Some(Item::Socket2Write)
        } else if self.has(Queue::SOCKET_2_READ) {
            self.clear_socket_2_transfer_data();
            Some(Item::Socket2Read)
        } else if self.has(Queue::WRITE_CONFIG) {
            log::info!("Triggering config write");
            self.clear(Queue::WRITE_CONFIG);
            Some(Item::WriteConfig)
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
            Some(Item::Socket1Close)
        } else if self.has(Queue::SOCKET_2_CLOSE) {
            self.clear(Queue::SOCKET_2_CLOSE);
            Some(Item::Socket2Close)
        } else if self.has(Queue::SOCKET_1_OPEN) {
            self.clear(Queue::SOCKET_1_OPEN);
            Some(Item::Socket1Open)
        } else if self.has(Queue::SOCKET_2_OPEN) {
            self.clear(Queue::SOCKET_2_OPEN);
            Some(Item::Socket2Open)
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

#[derive(Debug)]
pub(super) enum Item {
    Start,
    End,
    Reset,

    Connect,
    Disconnect,

    Socket1Open,
    Socket1Close,
    Socket2Open,
    Socket2Close,

    Socket1Read,
    Socket1Write,
    Socket2Read,
    Socket2Write,

    WriteConfig,

    Status,
    Idle,
}
