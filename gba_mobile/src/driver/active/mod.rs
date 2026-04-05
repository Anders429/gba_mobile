pub(crate) mod flow;
pub(crate) mod queue;

pub(in crate::driver) mod socket;

mod timeout;

pub(in crate::driver) use flow::Error;
pub(in crate::driver) use timeout::Timeout;

use crate::{
    ArrayVec, Config, Digit, Generation, Socket, Timer, dns,
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
        socket_requests: [Option<(SocketAddrV4, socket::Protocol)>; 2],
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
pub(super) struct Active<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: dns::Mode,
{
    queue: Queue<Socket1, Socket2, Dns>,
    flow: Option<Flow<Socket1, Socket2, Dns>>,

    state: State,
}

impl<Socket1, Socket2, Dns> Active<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: dns::Mode,
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

    pub(super) fn link_status(
        &self,
    ) -> Result<bool, super::error::link::Error<Socket1, Socket2, Dns>> {
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
    ) -> Result<bool, super::error::connection::Error<Socket1, Socket2, Dns>> {
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
    ) -> Result<usize, super::error::connection_io::Error<Buffer::ReadError, Socket1, Socket2, Dns>>
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
            Phase::Connected(_) => {
                let read_amount = socket
                    .read(buf)
                    .map_err(super::error::connection_io::Error::io)?;
                if socket.read_buffer.is_empty() {
                    // Schedule another transfer if the buffer is empty.
                    self.queue.set_socket_1_transfer();
                    // Accelerate the next automatic transfer.
                    socket.frame = u8::MAX;
                }
                Ok(read_amount)
            }
        }
    }

    pub(super) fn connection_write<Buffer>(
        &mut self,
        connection_generation: Generation,
        buf: &[u8],
        socket: &mut Socket<Buffer>,
    ) -> Result<usize, super::error::connection::Error<Socket1, Socket2, Dns>>
    where
        Buffer: crate::socket::Buffer,
    {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded().into());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded()),
            Phase::LoggedIn { .. } => Err(super::error::connection::Error::superseded()),
            Phase::Ending => Err(super::error::connection::Error::closed()),
            Phase::Connected(_) => {
                self.queue.set_socket_1_transfer();
                // Accelerate the next automatic transfer.
                socket.frame = u8::MAX;
                Ok(socket.write(buf))
            }
        }
    }

    pub(crate) fn open_socket<Buffer, const INDEX: usize>(
        &mut self,
        connection_generation: Generation,
        socket_addr: SocketAddrV4,
        protocol: socket::Protocol,
        socket: &mut crate::Socket<Buffer>,
    ) -> Result<Generation, super::error::connection::Error<Socket1, Socket2, Dns>> {
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
                socket_requests[INDEX] = Some((socket_addr, protocol));

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
    ) -> Result<bool, super::error::socket::Error<Socket1, Socket2, Dns>> {
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
    ) -> Result<usize, super::error::socket_io::Error<Buffer::ReadError, Socket1, Socket2, Dns>>
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
            Phase::Ending => Err(super::error::link::Error::closed().into()),
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
                        let read_amount = socket
                            .read(buf)
                            .map_err(super::error::socket_io::Error::io)?;
                        if socket.read_buffer.is_empty() {
                            // Schedule another transfer if the buffer is empty.
                            if INDEX == 0 {
                                self.queue.set_socket_1_transfer();
                            } else {
                                self.queue.set_socket_2_transfer();
                            }
                            // Accelerate the next automatic transfer.
                            socket.frame = u8::MAX;
                        }
                        Ok(read_amount)
                    }
                    crate::socket::Status::ConnectionFailure => Err(todo!()),
                    crate::socket::Status::ConnectionLost => Err(todo!()),
                    crate::socket::Status::ClosedRemotely => Err(todo!()),
                }
            }
        }
    }

    pub(super) fn socket_write<Buffer, const INDEX: usize>(
        &mut self,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &[u8],
        socket: &mut Socket<Buffer>,
    ) -> Result<usize, super::error::socket::Error<Socket1, Socket2, Dns>>
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
            Phase::Ending => Err(super::error::link::Error::closed().into()),
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
                        if INDEX == 0 {
                            self.queue.set_socket_1_transfer();
                        } else {
                            self.queue.set_socket_2_transfer();
                        }
                        // Accelerate the next automatic transfer.
                        socket.frame = u8::MAX;
                        Ok(socket.write(buf))
                    }
                    crate::socket::Status::ConnectionFailure => Err(todo!()),
                    crate::socket::Status::ConnectionLost => Err(todo!()),
                    crate::socket::Status::ClosedRemotely => Err(todo!()),
                }
            }
        }
    }

    pub(crate) fn dns<const MAX_LEN: usize>(
        &mut self,
        connection_generation: Generation,
        name: ArrayVec<u8, MAX_LEN>,
        dns: &mut crate::Dns<MAX_LEN>,
    ) -> Result<Generation, super::error::connection::Error<Socket1, Socket2, Dns>> {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded().into());
        }

        match &mut self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded()),
            Phase::Connected(_) => Err(super::error::connection::Error::superseded()),
            Phase::Ending => Err(super::error::link::Error::closed().into()),
            Phase::LoggedIn { .. } => {
                dns.state = dns::State::Request(name);
                self.queue.set_dns();
                dns.generation = dns.generation.increment();
                Ok(dns.generation)
            }
        }
    }

    pub(crate) fn dns_status<const MAX_LEN: usize>(
        &self,
        connection_generation: Generation,
        dns_generation: Generation,
        dns: &crate::Dns<MAX_LEN>,
    ) -> Result<Option<Ipv4Addr>, super::error::dns::Error<Socket1, Socket2, Dns>> {
        if self.state.connection_generation != connection_generation {
            return Err(super::error::connection::Error::superseded().into());
        }

        match &self.state.phase {
            Phase::Linking => Err(super::error::connection::Error::superseded().into()),
            Phase::Linked { .. } => Err(super::error::connection::Error::superseded().into()),
            Phase::Connecting(_) => Err(super::error::connection::Error::superseded().into()),
            Phase::Connected(_) => Err(super::error::connection::Error::superseded().into()),
            Phase::Ending => Err(super::error::link::Error::closed().into()),
            Phase::LoggedIn { .. } => {
                if dns.generation != dns_generation {
                    return Err(super::error::dns::Error::superseded());
                }

                match dns.state {
                    dns::State::Request(_) => Ok(None),
                    dns::State::Success(ip) => Ok(Some(ip)),
                    dns::State::NotFound => Err(super::error::dns::Error::not_found()),
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
    ) -> Result<Ipv4Addr, super::error::connection::Error<Socket1, Socket2, Dns>> {
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
    ) -> Result<Ipv4Addr, super::error::connection::Error<Socket1, Socket2, Dns>> {
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
    ) -> Result<Ipv4Addr, super::error::connection::Error<Socket1, Socket2, Dns>> {
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
        dns: &Dns,
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

                // Schedule a new data transfer once per second.
                //
                // This ensures any available data is received and available if the user requests
                // it.
                if socket_1.ready_for_transfer(frames::ONE_SECOND) {
                    self.queue.set_socket_1_transfer();
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

                // Schedule a new data transfer once every two seconds.
                //
                // This ensures any available data is received and available if the user requests
                // it.
                //
                // We use two seconds to give space for other requests. Otherwise, these high
                // priority requests would not allow anything else to execute when both sockets are
                // open.
                if socket_1.ready_for_transfer(frames::TWO_SECONDS) {
                    self.queue.set_socket_1_transfer();
                }
                if socket_2.ready_for_transfer(frames::TWO_SECONDS) {
                    self.queue.set_socket_2_transfer();
                }
            }
            _ => {}
        }

        if let Some(flow) = self.flow.take() {
            self.flow = Some(flow.vblank()?);
            Ok(())
        } else if let Some(new_flow) =
            self.queue
                .next_flow(&mut self.state, timer, socket_1, socket_2, dns)
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
        dns: &mut Dns,
    ) -> Result<StateChange, Error<Socket1, Socket2, Dns>> {
        if let Some(flow) = self.flow.take() {
            match flow.serial(
                &mut self.state,
                &mut self.queue,
                timer,
                socket_1,
                socket_2,
                dns,
            )? {
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
