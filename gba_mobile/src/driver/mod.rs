pub(crate) mod active;
pub(crate) mod error;

mod adapter;
mod command;
mod frames;
mod timers;

pub use adapter::Adapter;

use crate::{
    ArrayVec, Config, Digit, Dns, Generation, Socket, Timer, dns,
    mmio::{
        interrupt,
        serial::{self, RCNT, SIOCNT, TransferLength},
    },
    socket,
};
use active::Active;
use command::Command;
use core::net::{Ipv4Addr, SocketAddrV4};
use error::Error;

#[derive(Debug)]
enum State<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    /// Not currently linked with a Mobile Adapter device.
    Inactive,
    /// Currently linked with a Mobile Adapter device.
    Active(Active<Socket1, Socket2, Dns>),
    /// Communication encountered an error and the link must be reset.
    Error(Error<Socket1, Socket2, Dns>),
}

#[derive(Debug)]
pub struct Driver<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    link_generation: Generation,
    timer: Timer,

    socket_1: Socket1,
    socket_2: Socket2,
    dns: Dns,

    state: State<Socket1, Socket2, Dns>,
}

impl<Socket1, Socket2, Dns> Driver<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub const fn new(timer: Timer, socket_1: Socket1, socket_2: Socket2, dns: Dns) -> Self {
        Self {
            link_generation: Generation::new(),
            timer,

            socket_1,
            socket_2,
            dns,

            state: State::Inactive,
        }
    }

    /// Configures serial communication for a brand new link attempt.
    fn enable_communication() {
        unsafe {
            // Set transfer mode to 8-bit Normal.
            RCNT.write_volatile(serial::Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(TransferLength::_8Bit));
        }
    }

    /// Enable interrupts required for the driver to function.
    fn enable_interrupts(timer: Timer) {
        unsafe {
            // Enable interrupts for vblank, timer, and serial.
            let timer_enable = match timer {
                Timer::_0 => interrupt::Enable::TIMER0,
                Timer::_1 => interrupt::Enable::TIMER1,
                Timer::_2 => interrupt::Enable::TIMER2,
                Timer::_3 => interrupt::Enable::TIMER3,
            };
            interrupt::ENABLE.write_volatile(
                interrupt::ENABLE.read_volatile()
                    | interrupt::Enable::VBLANK
                    | timer_enable
                    | interrupt::Enable::SERIAL,
            );
        }
    }

    pub(crate) fn link(&mut self) -> Generation {
        self.link_generation = self.link_generation.increment();
        Self::enable_interrupts(self.timer);
        match &mut self.state {
            State::Inactive | State::Error(_) => {
                Self::enable_communication();
                self.state = State::Active(Active::new(self.link_generation));
            }
            State::Active(active) => {
                active.start_link();
            }
        }
        self.link_generation
    }

    pub(crate) fn as_active<'a>(
        &'a self,
        link_generation: Generation,
    ) -> Result<ActiveDriver<'a, Socket1, Socket2, Dns>, error::link::Error<Socket1, Socket2, Dns>>
    {
        if link_generation == self.link_generation {
            match &self.state {
                State::Inactive => Err(error::link::Error::closed()),
                State::Active(active) => Ok(ActiveDriver {
                    socket_1: &self.socket_1,
                    socket_2: &self.socket_2,
                    dns: &self.dns,

                    active,
                }),
                State::Error(error) => Err(error.clone().into()),
            }
        } else {
            Err(error::link::Error::superseded())
        }
    }

    pub(crate) fn as_active_mut<'a>(
        &'a mut self,
        link_generation: Generation,
    ) -> Result<ActiveDriverMut<'a, Socket1, Socket2, Dns>, error::link::Error<Socket1, Socket2, Dns>>
    {
        if link_generation == self.link_generation {
            match &mut self.state {
                State::Inactive => Err(error::link::Error::closed()),
                State::Active(active) => Ok(ActiveDriverMut {
                    socket_1: &mut self.socket_1,
                    socket_2: &mut self.socket_2,
                    dns: &mut self.dns,

                    active,
                }),
                State::Error(error) => Err(error.clone().into()),
            }
        } else {
            Err(error::link::Error::superseded())
        }
    }

    pub fn timer(&mut self) {
        match &mut self.state {
            State::Inactive => {}
            State::Active(active) => active.timer(self.timer),
            State::Error(_) => {}
        }
    }

    pub fn serial(&mut self) {
        match &mut self.state {
            State::Inactive => {}
            State::Active(active) => {
                if let Err(error) = active.serial(
                    self.timer,
                    self.link_generation,
                    &mut self.socket_1,
                    &mut self.socket_2,
                    &mut self.dns,
                ) {
                    self.state = State::Error(Error::Error(error));
                }
            }
            State::Error(_) => {}
        }
    }

    pub fn vblank(&mut self) {
        match &mut self.state {
            State::Inactive => {}
            State::Active(active) => {
                match active.vblank(
                    self.timer,
                    self.link_generation,
                    &mut self.socket_1,
                    &mut self.socket_2,
                    &self.dns,
                ) {
                    Ok(active::StateChange::StillActive) => {}
                    Ok(active::StateChange::Inactive) => self.state = State::Inactive,
                    Err(timeout) => self.state = State::Error(Error::Timeout(timeout)),
                }
            }
            State::Error(_) => {}
        }
    }
}

#[derive(Debug)]
pub(crate) struct ActiveDriver<'a, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    socket_1: &'a Socket1,
    socket_2: &'a Socket2,
    dns: &'a Dns,

    active: &'a Active<Socket1, Socket2, Dns>,
}

impl<'a, Socket1, Socket2, Dns> ActiveDriver<'a, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn link_status(self) -> Result<bool, error::link::Error<Socket1, Socket2, Dns>> {
        self.active.link_status()
    }

    pub(crate) fn connection_status(
        self,
        connection_generation: Generation,
    ) -> Result<bool, error::connection::Error<Socket1, Socket2, Dns>> {
        self.active.connection_status(connection_generation)
    }

    pub(crate) fn adapter(self) -> Result<Adapter, error::link::Error<Socket1, Socket2, Dns>> {
        self.active.adapter()
    }

    pub(crate) fn ip(
        self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error<Socket1, Socket2, Dns>> {
        self.active.ip(connection_generation)
    }

    pub(crate) fn primary_dns(
        self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error<Socket1, Socket2, Dns>> {
        self.active.primary_dns(connection_generation)
    }

    pub(crate) fn secondary_dns(
        self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error<Socket1, Socket2, Dns>> {
        self.active.secondary_dns(connection_generation)
    }

    pub(crate) fn config(self) -> Result<&'a [u8; 256], error::link::Error<Socket1, Socket2, Dns>> {
        self.active.config()
    }
}

impl<'a, Buffer, Socket2, Dns> ActiveDriver<'a, Socket<Buffer>, Socket2, Dns>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn socket_1_status(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<bool, error::socket::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active
            .socket_status::<_, 0>(connection_generation, socket_generation, self.socket_1)
    }
}

impl<'a, Buffer, Socket1, Dns> ActiveDriver<'a, Socket1, Socket<Buffer>, Dns>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn socket_2_status(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<bool, error::socket::Error<Socket1, Socket<Buffer>, Dns>> {
        self.active
            .socket_status::<_, 1>(connection_generation, socket_generation, &self.socket_2)
    }
}

impl<'a, Socket1, Socket2, const MAX_LEN: usize> ActiveDriver<'a, Socket1, Socket2, Dns<MAX_LEN>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub(crate) fn dns_status(
        self,
        connection_generation: Generation,
        dns_generation: Generation,
    ) -> Result<Option<Ipv4Addr>, error::dns::Error<Socket1, Socket2, Dns<MAX_LEN>>> {
        self.active
            .dns_status(connection_generation, dns_generation, &self.dns)
    }
}

#[derive(Debug)]
pub(crate) struct ActiveDriverMut<'a, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    socket_1: &'a mut Socket1,
    socket_2: &'a mut Socket2,
    dns: &'a mut Dns,

    active: &'a mut Active<Socket1, Socket2, Dns>,
}

impl<'a, Socket1, Socket2, Dns> ActiveDriverMut<'a, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn close_link(self) -> Result<(), error::link::Error<Socket1, Socket2, Dns>> {
        self.active.close_link()
    }

    pub(crate) fn login(
        &mut self,
        phone_number: ArrayVec<Digit, 32>,
        id: ArrayVec<u8, 32>,
        password: ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Result<Generation, error::link::Error<Socket1, Socket2, Dns>> {
        self.active
            .login(phone_number, id, password, primary_dns, secondary_dns)
    }

    pub(crate) fn disconnect(
        self,
        connection_generation: Generation,
    ) -> Result<(), error::connection::Error<Socket1, Socket2, Dns>> {
        self.active.disconnect(connection_generation)
    }

    pub(crate) fn write_config<Config>(
        &mut self,
        config: Config,
    ) -> Result<(), error::link::Error<Socket1, Socket2, Dns>>
    where
        Config: self::Config,
    {
        self.active.write_config(config)
    }
}

impl<'a, Buffer, Socket2, Dns> ActiveDriverMut<'a, Socket<Buffer>, Socket2, Dns>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn accept(
        self,
    ) -> Result<Generation, error::link::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active.accept()
    }

    pub(crate) fn connect(
        self,
        phone_number: ArrayVec<Digit, 32>,
    ) -> Result<Generation, error::link::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active.connect(phone_number)
    }

    pub(crate) fn open_tcp_1(
        self,
        connection_generation: Generation,
        socket_addr: SocketAddrV4,
    ) -> Result<Generation, error::connection::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active.open_socket::<_, 0>(
            connection_generation,
            socket_addr,
            socket::Protocol::Tcp,
            self.socket_1,
        )
    }

    pub(crate) fn open_udp_1(
        self,
        connection_generation: Generation,
        socket_addr: SocketAddrV4,
    ) -> Result<Generation, error::connection::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active.open_socket::<_, 0>(
            connection_generation,
            socket_addr,
            socket::Protocol::Udp,
            self.socket_1,
        )
    }

    pub(crate) fn close_socket_1(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<(), error::socket::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active
            .close_socket::<_, 0>(connection_generation, socket_generation, self.socket_1)
    }

    pub(crate) fn connection_read(
        self,
        connection_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, error::connection_io::Error<Buffer::ReadError, Socket<Buffer>, Socket2, Dns>>
    {
        self.active
            .connection_read(connection_generation, buf, self.socket_1)
    }

    pub(crate) fn connection_write(
        self,
        connection_generation: Generation,
        buf: &[u8],
    ) -> Result<usize, error::connection::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active
            .connection_write(connection_generation, buf, self.socket_1)
    }

    pub(crate) fn connection_flush(
        self,
        connection_generation: Generation,
    ) -> Result<(), error::connection::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active
            .connection_flush(connection_generation, self.socket_1)
    }

    pub(crate) fn socket_1_read(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, error::socket_io::Error<Buffer::ReadError, Socket<Buffer>, Socket2, Dns>>
    {
        self.active.socket_read::<_, 0>(
            connection_generation,
            socket_generation,
            buf,
            self.socket_1,
        )
    }

    pub(crate) fn socket_1_write(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &[u8],
    ) -> Result<usize, error::socket::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active.socket_write::<_, 0>(
            connection_generation,
            socket_generation,
            buf,
            self.socket_1,
        )
    }

    pub(crate) fn socket_1_flush(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<(), error::socket::Error<Socket<Buffer>, Socket2, Dns>> {
        self.active
            .socket_flush::<_, 0>(connection_generation, socket_generation, self.socket_1)
    }
}

impl<'a, Buffer, Socket1, Dns> ActiveDriverMut<'a, Socket1, Socket<Buffer>, Dns>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn open_tcp_2(
        self,
        connection_generation: Generation,
        socket_addr: SocketAddrV4,
    ) -> Result<Generation, error::connection::Error<Socket1, Socket<Buffer>, Dns>> {
        self.active.open_socket::<_, 1>(
            connection_generation,
            socket_addr,
            socket::Protocol::Tcp,
            self.socket_2,
        )
    }

    pub(crate) fn open_udp_2(
        self,
        connection_generation: Generation,
        socket_addr: SocketAddrV4,
    ) -> Result<Generation, error::connection::Error<Socket1, Socket<Buffer>, Dns>> {
        self.active.open_socket::<_, 1>(
            connection_generation,
            socket_addr,
            socket::Protocol::Udp,
            self.socket_2,
        )
    }

    pub(crate) fn close_socket_2(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<(), error::socket::Error<Socket1, Socket<Buffer>, Dns>> {
        self.active
            .close_socket::<_, 1>(connection_generation, socket_generation, self.socket_2)
    }

    pub(crate) fn socket_2_read(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, error::socket_io::Error<Buffer::ReadError, Socket1, Socket<Buffer>, Dns>>
    {
        self.active.socket_read::<_, 1>(
            connection_generation,
            socket_generation,
            buf,
            self.socket_2,
        )
    }

    pub(crate) fn socket_2_write(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &[u8],
    ) -> Result<usize, error::socket::Error<Socket1, Socket<Buffer>, Dns>> {
        self.active.socket_write::<_, 1>(
            connection_generation,
            socket_generation,
            buf,
            self.socket_2,
        )
    }

    pub(crate) fn socket_2_flush(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<(), error::socket::Error<Socket1, Socket<Buffer>, Dns>> {
        self.active
            .socket_flush::<_, 1>(connection_generation, socket_generation, self.socket_2)
    }
}

impl<'a, Socket1, Socket2, const MAX_LEN: usize> ActiveDriverMut<'a, Socket1, Socket2, Dns<MAX_LEN>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub(crate) fn dns(
        self,
        connection_generation: Generation,
        name: ArrayVec<u8, MAX_LEN>,
    ) -> Result<Generation, error::connection::Error<Socket1, Socket2, Dns<MAX_LEN>>> {
        self.active.dns(connection_generation, name, self.dns)
    }

    pub(crate) fn cancel_dns(
        self,
        connection_generation: Generation,
        dns_generation: Generation,
    ) -> Result<(), error::dns::Error<Socket1, Socket2, Dns<MAX_LEN>>> {
        self.active
            .cancel_dns(connection_generation, dns_generation, self.dns)
    }
}
