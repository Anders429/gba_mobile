pub(crate) mod flow;
pub(crate) mod queue;

pub(in crate::driver) mod socket;

mod timeout;

pub(in crate::driver) use flow::Error;
pub(in crate::driver) use timeout::Timeout;

use crate::{
    ArrayVec, Config, Digit, Generation, Socket, Timer,
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
        socket_requests: [Option<(socket::Request, socket::Protocol)>; 2],
    },

    /// This link is being closed.
    Ending,
}

#[derive(Debug)]
struct State {
    connection_generation: Generation,

    transfer_length: TransferLength,
    adapter: Adapter,

    phase: Phase,
    config: [u8; 256],

    frame: u8,
}

impl State {
    fn new() -> Self {
        Self {
            connection_generation: Generation::new(),

            transfer_length: TransferLength::_8Bit,
            // Arbitrary default. It will be overwritten after the first packet is received.
            adapter: Adapter::Blue,

            phase: Phase::Linking,
            config: [0; 256],

            frame: 0,
        }
    }
}

#[derive(Debug)]
pub(super) struct Active<Socket1, Socket2>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
{
    queue: Queue<Socket1, Socket2>,
    flow: Option<Flow<Socket1, Socket2>>,

    state: State,
}

impl<Socket1, Socket2> Active<Socket1, Socket2>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
{
    /// Define a new active communication state, attempting to immediately link with the Mobile
    /// Adapter.
    pub(super) fn new() -> Self {
        Self {
            queue: Queue::new(),
            flow: Some(Flow::start(TransferLength::_8Bit)),

            state: State::new(),
        }
    }

    /// Start a new link, closing any existing link if one is active.
    pub(super) fn start_link(&mut self) {
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

    pub(super) fn connection_read<Buffer>(
        &mut self,
        connection_generation: Generation,
        buf: &mut [u8],
        socket: &mut Socket<Buffer>,
    ) -> Result<usize, super::error::connection_io::Error<Buffer::ReadError>>
    where
        Buffer: crate::socket::Buffer,
    {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded().into());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded().into()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded().into()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded().into()),
            Phase::LoggedIn { .. } => Err(super::error::connection::Error::superseded().into()),
            Phase::Ending => Err(super::error::connection::Error::closed().into()),
            Phase::Connected(_) => socket
                .read(buf)
                .map_err(super::error::connection_io::Error::io),
        }
    }

    pub(crate) fn open_socket<Buffer, const INDEX: usize>(
        &mut self,
        connection_generation: Generation,
        host: Either<Ipv4Addr, ArrayVec<u8, 255>>,
        port: u16,
        protocol: socket::Protocol,
        socket: &mut crate::Socket<Buffer>,
    ) -> Result<Generation, super::error::connection::Error> {
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
                socket_requests,
                ..
            } => {
                socket.status = crate::socket::Status::Connecting;
                let request = match host {
                    Either::Left(ip) => socket::Request::SocketAddr(SocketAddrV4::new(ip, port)),
                    Either::Right(domain) => socket::Request::Dns { domain, port },
                };
                socket_requests[INDEX] = Some((request, protocol));

                if INDEX == 0 {
                    self.queue.set_socket_1_open();
                } else {
                    self.queue.set_socket_2_open();
                }

                socket_generations[INDEX] = socket_generations[INDEX].increment();
                Ok(socket_generations[INDEX])
            }
        }
    }

    pub(crate) fn socket_status<Buffer, const INDEX: usize>(
        &self,
        connection_generation: Generation,
        socket_generation: Generation,
        socket: &crate::Socket<Buffer>,
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
                socket_generations, ..
            } => {
                if socket_generations[INDEX] != socket_generation {
                    return Err(super::error::socket::Error::superseded());
                }

                match socket.status {
                    crate::socket::Status::NotConnected => {
                        Err(super::error::socket::Error::superseded())
                    }
                    crate::socket::Status::Connecting => Ok(false),
                    crate::socket::Status::Connected => Ok(true),
                    crate::socket::Status::ConnectionFailure => Err(todo!()),
                    crate::socket::Status::ConnectionLost => Err(todo!()),
                    crate::socket::Status::ClosedRemotely => Err(todo!()),
                }
            }
        }
    }

    pub(super) fn socket_read<Buffer, const INDEX: usize>(
        &mut self,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &mut [u8],
        socket: &mut Socket<Buffer>,
    ) -> Result<usize, super::error::socket_io::Error<Buffer::ReadError>>
    where
        Buffer: crate::socket::Buffer,
    {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded().into());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded().into()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded().into()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded().into()),
            Phase::Connected(_) => Err(super::error::connection::Error::superseded().into()),
            Phase::Ending => Err(super::error::connection::Error::closed().into()),
            Phase::LoggedIn {
                socket_generations, ..
            } => {
                if socket_generations[INDEX] != socket_generation {
                    return Err(super::error::socket::Error::superseded().into());
                }

                match socket.status {
                    crate::socket::Status::NotConnected => {
                        Err(super::error::socket::Error::superseded().into())
                    }
                    crate::socket::Status::Connecting => {
                        Err(super::error::socket::Error::superseded().into())
                    }
                    crate::socket::Status::Connected => {
                        socket.read(buf).map_err(super::error::socket_io::Error::io)
                    }
                    crate::socket::Status::ConnectionFailure => Err(todo!()),
                    crate::socket::Status::ConnectionLost => Err(todo!()),
                    crate::socket::Status::ClosedRemotely => Err(todo!()),
                }
            }
        }
    }

    pub(crate) fn adapter(&self) -> Adapter {
        self.state.adapter
    }

    pub(crate) fn ip(
        &self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, super::error::connection::Error> {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded()),
            Phase::Connected(_) => Err(super::error::connection::Error::superseded()),
            Phase::Ending => Err(super::error::link::Error::closed().into()),
            Phase::LoggedIn { ip, .. } => Ok(*ip),
        }
    }

    pub(crate) fn primary_dns(
        &self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, super::error::connection::Error> {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded()),
            Phase::Connected(_) => Err(super::error::connection::Error::superseded()),
            Phase::Ending => Err(super::error::link::Error::closed().into()),
            Phase::LoggedIn { primary_dns, .. } => Ok(*primary_dns),
        }
    }

    pub(crate) fn secondary_dns(
        &self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, super::error::connection::Error> {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded()),
            Phase::Connected(_) => Err(super::error::connection::Error::superseded()),
            Phase::Ending => Err(super::error::link::Error::closed().into()),
            Phase::LoggedIn { secondary_dns, .. } => Ok(*secondary_dns),
        }
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

    pub(super) fn vblank(
        &mut self,
        timer: Timer,
        socket_1: &mut Socket1,
        socket_2: &mut Socket2,
    ) -> Result<(), Timeout> {
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
            Phase::Connected(frame) => {
                if *frame == frames::ONE_SECOND {
                    // Schedule a new status flow once per second.
                    //
                    // This ensures we are constantly aware of whether the connection is still
                    // live.
                    self.queue.set_status();
                }
                *frame = frame.saturating_add(1);

                // TODO: Require that the receive buffer is empty.
                if let Some((frame, status)) = socket_1.vblank_info() {
                    if matches!(status, crate::socket::Status::Connected) {
                        if *frame == frames::ONE_SECOND {
                            // Schedule a new data transfer once per second.
                            //
                            // This ensures any available data is received and available if the
                            // user requests it.
                            self.queue.set_socket_1_transfer();
                        }
                        *frame = frame.saturating_add(1);
                    }
                }
            }
            Phase::LoggedIn { frame, .. } => {
                if *frame == frames::ONE_SECOND {
                    // Schedule a new status flow once per second.
                    //
                    // This ensures we are constantly aware of whether the connection is still
                    // live.
                    self.queue.set_status();
                }
                *frame = frame.saturating_add(1);

                if let Some((frame, status)) = socket_1.vblank_info() {
                    if matches!(status, crate::socket::Status::Connected) {
                        if *frame == frames::TWO_SECONDS {
                            // Schedule a new data transfer once every two seconds.
                            //
                            // This ensures any available data is received and available if the
                            // user requests it.
                            //
                            // We use two seconds to give space for other requests. Otherwise,
                            // these high priority requests would not allow anything else to
                            // execute when both sockets are open.
                            self.queue.set_socket_1_transfer();
                        }
                        *frame = frame.saturating_add(1);
                    }
                }

                if let Some((frame, status)) = socket_2.vblank_info() {
                    if matches!(status, crate::socket::Status::Connected) {
                        if *frame == frames::TWO_SECONDS {
                            // Schedule a new data transfer once every two seconds.
                            //
                            // This ensures any available data is received and available if the
                            // user requests it.
                            //
                            // We use two seconds to give space for other requests. Otherwise,
                            // these high priority requests would not allow anything else to
                            // execute when both sockets are open.
                            self.queue.set_socket_2_transfer();
                        }
                        *frame = frame.saturating_add(1);
                    }
                }
            }
            _ => {}
        }

        if let Some(flow) = self.flow.take() {
            self.flow = Some(flow.vblank()?);
            Ok(())
        } else if let Some(new_flow) =
            self.queue
                .next_flow(&mut self.state, timer, socket_1, socket_2)
        {
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

    pub(super) fn timer(&mut self, timer: Timer) {
        timer.stop();
        if let Some(flow) = &mut self.flow {
            flow.timer()
        }
    }

    pub(super) fn serial(
        &mut self,
        timer: Timer,
        socket_1: &mut Socket1,
        socket_2: &mut Socket2,
    ) -> Result<StateChange, Error> {
        if let Some(flow) = self.flow.take() {
            match flow.serial(&mut self.state, &mut self.queue, timer, socket_1, socket_2)? {
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
