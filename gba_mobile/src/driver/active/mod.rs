pub(in crate::driver) mod socket;

mod flow;
mod queue;
mod timeout;

pub(in crate::driver) use flow::Error;
pub(in crate::driver) use timeout::Timeout;

use crate::{
    ArrayVec, Config, Digit, Generation, Timer,
    driver::{Adapter, frames},
    mmio::serial::TransferLength,
};
use core::{
    fmt::{self, Display, Formatter},
    net::{Ipv4Addr, SocketAddrV4},
};
use either::Either;
use flow::Flow;
use queue::Queue;
use socket::Socket;

#[derive(Debug)]
enum ConnectionRequest {
    Accept {
        frame: u8,
    },
    Connect {
        phone_number: ArrayVec<Digit, 32>,
    },
    Login {
        phone_number: ArrayVec<Digit, 32>,
        id: ArrayVec<u8, 32>,
        password: ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    },
}

#[derive(Clone, Debug)]
pub(in crate::driver) enum ConnectionFailure {
    Connect,
    Login,
    LostConnection,
}

impl Display for ConnectionFailure {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Connect => formatter.write_str("unable to connect"),
            Self::Login => formatter.write_str("unable to login"),
            Self::LostConnection => formatter.write_str("lost connection"),
        }
    }
}

impl core::error::Error for ConnectionFailure {}

#[derive(Debug)]
enum Phase {
    /// Attempting to link with a Mobile Adapter device.
    Linking,
    /// Linked with a Mobile Adapter device.
    Linked {
        frame: u8,
        connection_failure: Option<ConnectionFailure>,
    },

    /// Attempting to establish a connection.
    Connecting(ConnectionRequest),
    /// Connection established.
    Connected(u8),
    // Logged in to PPP.
    LoggedIn {
        frame: u8,
        ip: Ipv4Addr,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
        socket_generations: [Generation; 2],
        socket_states: [socket::State; 2],
    },

    /// This link is being closed.
    Ending,
}

#[derive(Debug)]
struct State {
    connection_generation: Generation,

    transfer_length: TransferLength,
    adapter: Adapter,
    timer: Timer,

    phase: Phase,
    sockets: [Socket; 2],
    config: [u8; 256],

    frame: u8,
}

impl State {
    fn new(timer: Timer) -> Self {
        Self {
            connection_generation: Generation::new(),

            transfer_length: TransferLength::_8Bit,
            // Arbitrary default. It will be overwritten after the first packet is received.
            adapter: Adapter::Blue,
            timer,

            phase: Phase::Linking,
            sockets: [Socket::new(), Socket::new()],
            config: [0; 256],

            frame: 0,
        }
    }
}

#[derive(Debug)]
pub(super) struct Active {
    queue: Queue,
    flow: Option<Flow>,

    state: State,
}

impl Active {
    /// Define a new active communication state, attempting to immediately link with the Mobile
    /// Adapter.
    pub(super) fn new(timer: Timer) -> Self {
        Self {
            queue: Queue::new(),
            flow: Some(Flow::start(TransferLength::_8Bit)),

            state: State::new(timer),
        }
    }

    /// Start a new link, closing any existing link if one is active.
    pub(super) fn start_link(&mut self, timer: Timer) {
        match self.state.phase {
            Phase::Linking | Phase::Ending => {
                // In either of these phases, we do not need to schedule the end of the previous
                // session, since it either doesn't exist or is already scheduled to end.
                self.queue.set_start();
            }
            _ => {
                self.queue.set_end();
                self.queue.set_start();
            }
        }
        self.state.phase = Phase::Linking;
        self.state.timer = timer;
    }

    pub(super) fn link_status(&self) -> Result<bool, super::error::link::Error> {
        match self.state.phase {
            Phase::Linking => Ok(false),
            Phase::Ending => Err(super::error::link::Error::closed()),
            _ => Ok(true),
        }
    }

    /// End the existing session.
    pub(super) fn close_link(&mut self) {
        self.queue.set_end();
        self.state.phase = Phase::Ending;
    }

    /// Listen for an incoming p2p connection.
    pub(super) fn accept(&mut self) -> Generation {
        self.state.connection_generation = self.state.connection_generation.increment();
        if matches!(
            self.state.phase,
            Phase::Connecting(_) | Phase::Connected(_) | Phase::LoggedIn { .. }
        ) {
            // If we are already connected or attempting to connect, disconnect first.
            self.queue.set_disconnect();
        }
        self.state.phase = Phase::Connecting(ConnectionRequest::Accept { frame: 255 });
        self.queue.set_connect();
        self.state.connection_generation
    }

    /// Connect to a p2p peer.
    pub(super) fn connect(&mut self, phone_number: ArrayVec<Digit, 32>) -> Generation {
        self.state.connection_generation = self.state.connection_generation.increment();
        if matches!(
            self.state.phase,
            Phase::Connecting(_) | Phase::Connected(_) | Phase::LoggedIn { .. }
        ) {
            // If we are already connected or attempting to connect, disconnect first.
            self.queue.set_disconnect();
        }
        self.state.phase = Phase::Connecting(ConnectionRequest::Connect { phone_number });
        self.queue.set_connect();
        self.state.connection_generation
    }

    /// Connect via PPP protocol.
    pub(super) fn login(
        &mut self,
        phone_number: ArrayVec<Digit, 32>,
        id: ArrayVec<u8, 32>,
        password: ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Generation {
        self.state.connection_generation = self.state.connection_generation.increment();
        if matches!(
            self.state.phase,
            Phase::Connecting(_) | Phase::Connected(_) | Phase::LoggedIn { .. }
        ) {
            // If we are already connected or attempting to connect, disconnect first.
            self.queue.set_disconnect();
        }
        self.state.phase = Phase::Connecting(ConnectionRequest::Login {
            phone_number,
            id,
            password,
            primary_dns,
            secondary_dns,
        });
        self.queue.set_connect();
        self.state.connection_generation
    }

    pub(crate) fn connection_status(
        &self,
        connection_generation: Generation,
    ) -> Result<bool, super::error::connection::Error> {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded()),
            Phase::Linked {
                connection_failure: Some(failure),
                ..
            } => Err(failure.clone().into()),
            Phase::Linked {
                connection_failure: None,
                ..
            } => Err(super::error::connection::Error::superseded()),
            Phase::Connecting(_) => Ok(false),
            Phase::Ending => Err(super::error::link::Error::closed().into()),
            _ => Ok(true),
        }
    }

    pub(crate) fn open_socket(
        &mut self,
        connection_generation: Generation,
        host: Either<Ipv4Addr, ArrayVec<u8, 255>>,
        port: u16,
        protocol: socket::Protocol,
    ) -> Result<Option<(Generation, crate::socket::Index)>, super::error::connection::Error> {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded().into());
        }

        match &mut self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded()),
            Phase::Connected(_) => Err(super::error::connection::Error::superseded()),
            Phase::Ending => Err(super::error::link::Error::closed().into()),
            Phase::LoggedIn {
                socket_generations,
                socket_states,
                ..
            } => {
                // Find the first available socket.
                let mut socket_index = None;
                for (index, socket_state) in socket_states.iter().enumerate() {
                    if matches!(
                        socket_state,
                        socket::State::Available | socket::State::Failure(_)
                    ) {
                        if index == 0 {
                            socket_index = Some(crate::socket::Index::One);
                        } else {
                            socket_index = Some(crate::socket::Index::Two);
                        }
                        break;
                    }
                }
                if let Some(socket_index) = socket_index {
                    // Schedule the request.
                    let request = match host {
                        Either::Left(ip) => {
                            socket::Request::SocketAddr(SocketAddrV4::new(ip, port))
                        }
                        Either::Right(domain) => socket::Request::Dns { domain, port },
                    };
                    socket_states[usize::from(socket_index)] =
                        socket::State::Connecting(request, protocol);
                    if matches!(socket_index, crate::socket::Index::One) {
                        self.queue.set_socket_1_open();
                    } else {
                        self.queue.set_socket_2_open();
                    }

                    // Return the new socket generation.
                    socket_generations[usize::from(socket_index)].increment();
                    Ok(Some((
                        socket_generations[usize::from(socket_index)],
                        socket_index,
                    )))
                } else {
                    // There are no unused sockets.
                    Ok(None)
                }
            }
        }
    }

    pub(crate) fn socket_status(
        &self,
        connection_generation: Generation,
        socket_generation: Generation,
        index: crate::socket::Index,
    ) -> Result<bool, super::error::socket::Error> {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded().into());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded().into()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded().into()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded().into()),
            Phase::Connected(_) => Err(super::error::connection::Error::superseded().into()),
            Phase::Ending => Err(super::error::link::Error::closed().into()),
            Phase::LoggedIn {
                socket_generations,
                socket_states,
                ..
            } => {
                if socket_generations[usize::from(index)] != socket_generation {
                    return Err(super::error::socket::Error::superseded());
                }

                match &socket_states[usize::from(index)] {
                    socket::State::Available => Err(super::error::socket::Error::superseded()),
                    socket::State::Connecting(_, _) => Ok(false),
                    socket::State::Connected => Ok(true),
                    socket::State::Failure(error) => Err(error.clone().into()),
                }
            }
        }
    }

    pub(crate) fn adapter(&self) -> Adapter {
        self.state.adapter
    }

    pub(crate) fn config(&self) -> &[u8; 256] {
        &self.state.config
    }

    pub(crate) fn write_config<Config>(&mut self, config: Config)
    where
        Config: self::Config,
    {
        // Clear config before writing to it.
        //
        // We don't require config formats to guarantee that they overwrite every byte.
        self.state.config.fill(0);

        config.write(&mut self.state.config);
        self.queue.set_write_config();
    }

    pub(super) fn vblank(&mut self) -> Result<(), Timeout> {
        match &mut self.state.phase {
            Phase::Linked { frame, .. } => {
                if *frame == frames::ONE_SECOND {
                    // Schedule a new idle pulse once per second.
                    //
                    // This ensures the link stays alive, despite us not sending any packet
                    // requests.
                    self.queue.set_idle();
                }
                *frame = frame.saturating_add(1);
            }
            Phase::Connecting(ConnectionRequest::Accept { frame }) => {
                if *frame == frames::ONE_SECOND {
                    // Schedule a new connection attempt every second.
                    self.queue.set_connect();
                }
                *frame = frame.saturating_add(1);
            }
            Phase::Connected(frame) | Phase::LoggedIn { frame, .. } => {
                if *frame == frames::ONE_SECOND {
                    // Schedule a new status flow once per second.
                    //
                    // This ensures we are constantly aware of whether the connection is still
                    // live.
                    self.queue.set_status();
                }
                *frame = frame.saturating_add(1);
            }
            _ => {}
        }

        if let Some(flow) = self.flow.take() {
            self.flow = Some(flow.vblank()?);
            Ok(())
        } else if let Some(new_flow) = self.queue.next_flow(&self.state) {
            // Reset the frame count so we don't timeout.
            self.state.frame = 0;
            self.flow = Some(new_flow);
            Ok(())
        } else if self.state.frame > frames::THREE_SECONDS {
            // Three seconds is how long the adapter will remain connected without any bytes
            // sent to it, so this timeout should align with the disconnect.
            Err(Timeout::Queue)
        } else {
            // No flow being processed and none on the queue. Increment the frame so that we
            // timeout if we remain in this state too long.
            self.state.frame += 1;
            Ok(())
        }
    }

    pub(super) fn timer(&mut self) {
        self.state.timer.stop();
        if let Some(flow) = &mut self.flow {
            flow.timer()
        }
    }

    pub(super) fn serial(&mut self) -> Result<StateChange, Error> {
        if let Some(flow) = self.flow.take() {
            match flow.serial(&mut self.state, &mut self.queue)? {
                Either::Left(flow) => {
                    self.flow = Some(flow);
                    Ok(StateChange::StillActive)
                }
                Either::Right(state_change) => Ok(state_change),
            }
        } else {
            Ok(StateChange::StillActive)
        }
    }
}

#[derive(Debug)]
pub(in crate::driver) enum StateChange {
    StillActive,
    Inactive,
}
