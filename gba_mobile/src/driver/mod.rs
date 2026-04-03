pub(crate) mod active;
pub(crate) mod error;

mod adapter;
mod command;
mod frames;
mod timers;

pub use adapter::Adapter;

use crate::{
    ArrayVec, Config, Digit, Generation, Socket, Timer,
    mmio::{
        interrupt,
        serial::{self, RCNT, SIOCNT, TransferLength},
    },
    socket,
};
use active::Active;
use command::Command;
use core::net::Ipv4Addr;
use either::Either;
use embedded_io::{Read, Write};
use error::Error;

#[derive(Debug)]
enum State<Socket1, Socket2>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    /// Not currently linked with a Mobile Adapter device.
    Inactive,
    /// Currently linked with a Mobile Adapter device.
    Active(Active<Socket1, Socket2>),
    /// Communication encountered an error and the link must be reset.
    Error(Error),
}

#[derive(Debug)]
pub struct Driver<Socket1, Socket2>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    link_generation: Generation,
    timer: Timer,

    socket_1: Socket1,
    socket_2: Socket2,

    state: State<Socket1, Socket2>,
}

impl<Socket1, Socket2> Driver<Socket1, Socket2>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub const fn new(timer: Timer, socket_1: Socket1, socket_2: Socket2) -> Self {
        Self {
            link_generation: Generation::new(),
            timer,

            socket_1,
            socket_2,

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
                self.state = State::Active(Active::new());
            }
            State::Active(active) => {
                active.start_link();
            }
        }
        self.link_generation
    }

    pub(crate) fn link_status(
        &self,
        link_generation: Generation,
    ) -> Result<bool, error::link::Error> {
        if link_generation != self.link_generation {
            return Err(error::link::Error::superseded());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed()),
            State::Active(active) => active.link_status(),
            State::Error(error) => Err(error.clone().into()),
        }
    }

    pub(crate) fn close_link(&mut self, link_generation: Generation) {
        if self.link_generation != link_generation {
            // This request came from an old link. We should not honor it, as that link is already
            // closed.
            return;
        }

        match &mut self.state {
            State::Inactive | State::Error(_) => {
                self.state = State::Inactive;
            }
            State::Active(active) => {
                active.close_link();
            }
        }
    }

    pub(crate) fn close_link_status(
        &self,
        link_generation: Generation,
    ) -> Result<bool, error::close_link::Error> {
        if self.link_generation != link_generation {
            return Err(error::close_link::Error::superseded());
        }

        match &self.state {
            State::Inactive => Ok(true),
            State::Active(_) => Ok(false),
            State::Error(error) => Err(error.clone().into()),
        }
    }

    pub(crate) fn login(
        &mut self,
        link_generation: Generation,
        phone_number: ArrayVec<Digit, 32>,
        id: ArrayVec<u8, 32>,
        password: ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Result<Generation, error::link::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed()),
            State::Active(active) => {
                Ok(active.login(phone_number, id, password, primary_dns, secondary_dns))
            }
            State::Error(error) => Err(error.clone().into()),
        }
    }

    pub(crate) fn connection_status(
        &self,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<bool, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.connection_status(connection_generation),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn disconnect(&mut self) {
        todo!()
    }

    pub(crate) fn adapter(
        &self,
        link_generation: Generation,
    ) -> Result<Adapter, error::link::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => Ok(active.adapter()),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn ip(
        &self,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.ip(connection_generation),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn primary_dns(
        &self,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.primary_dns(connection_generation),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn secondary_dns(
        &self,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.secondary_dns(connection_generation),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn config(
        &self,
        link_generation: Generation,
    ) -> Result<&[u8; 256], error::link::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => Ok(active.config()),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn write_config<Config>(
        &mut self,
        link_generation: Generation,
        config: Config,
    ) -> Result<(), error::link::Error>
    where
        Config: self::Config,
    {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => Ok(active.write_config(config)),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
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
                match active.serial(self.timer, &mut self.socket_1, &mut self.socket_2) {
                    Ok(active::StateChange::StillActive) => {}
                    Ok(active::StateChange::Inactive) => self.state = State::Inactive,
                    Err(error) => self.state = State::Error(Error::Error(error)),
                }
            }
            State::Error(_) => {}
        }
    }
}

impl<'a, Socket1, Socket2> Driver<Socket1, Socket2>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
{
    pub fn vblank(&mut self) {
        match &mut self.state {
            State::Inactive => {}
            State::Active(active) => {
                if let Err(timeout) =
                    active.vblank(self.timer, &mut self.socket_1, &mut self.socket_2)
                {
                    self.state = State::Error(Error::Timeout(timeout));
                }
            }
            State::Error(_) => {}
        }
    }
}

impl<Buffer, Socket2> Driver<Socket<Buffer>, Socket2>
where
    Buffer: Read + Write,
    Socket2: socket::Slot,
{
    pub(crate) fn accept(
        &mut self,
        link_generation: Generation,
    ) -> Result<Generation, error::link::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed()),
            State::Active(active) => Ok(active.accept()),
            State::Error(error) => Err(error.clone().into()),
        }
    }

    pub(crate) fn connect(
        &mut self,
        link_generation: Generation,
        phone_number: ArrayVec<Digit, 32>,
    ) -> Result<Generation, error::link::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed()),
            State::Active(active) => Ok(active.connect(phone_number)),
            State::Error(error) => Err(error.clone().into()),
        }
    }

    pub(crate) fn open_tcp_1(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        host: Either<Ipv4Addr, ArrayVec<u8, 255>>,
        port: u16,
    ) -> Result<Generation, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.open_socket::<_, 0>(
                connection_generation,
                host,
                port,
                active::socket::Protocol::Tcp,
                &mut self.socket_1,
            ),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn open_udp_1(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        host: Either<Ipv4Addr, ArrayVec<u8, 255>>,
        port: u16,
    ) -> Result<Generation, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.open_socket::<_, 0>(
                connection_generation,
                host,
                port,
                active::socket::Protocol::Udp,
                &mut self.socket_1,
            ),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn socket_1_status(
        &self,
        link_generation: Generation,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<bool, error::socket::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.socket_status::<_, 0>(
                connection_generation,
                socket_generation,
                &self.socket_1,
            ),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn connection_read(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, error::connection_io::Error<Buffer::Error>> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => {
                active.connection_read(connection_generation, buf, &mut self.socket_1)
            }
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn socket_1_read(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, error::socket_io::Error<Buffer::Error>> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.socket_read::<_, 0>(
                connection_generation,
                socket_generation,
                buf,
                &mut self.socket_1,
            ),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }
}

impl<Buffer, Socket1> Driver<Socket1, Socket<Buffer>>
where
    Buffer: Read + Write,
    Socket1: socket::Slot,
{
    pub(crate) fn open_tcp_2(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        host: Either<Ipv4Addr, ArrayVec<u8, 255>>,
        port: u16,
    ) -> Result<Generation, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.open_socket::<_, 1>(
                connection_generation,
                host,
                port,
                active::socket::Protocol::Tcp,
                &mut self.socket_2,
            ),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn open_udp_2(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        host: Either<Ipv4Addr, ArrayVec<u8, 255>>,
        port: u16,
    ) -> Result<Generation, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.open_socket::<_, 1>(
                connection_generation,
                host,
                port,
                active::socket::Protocol::Udp,
                &mut self.socket_2,
            ),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn socket_2_status(
        &self,
        link_generation: Generation,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<bool, error::socket::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.socket_status::<_, 1>(
                connection_generation,
                socket_generation,
                &self.socket_2,
            ),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn socket_2_read(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &mut [u8],
    ) -> Result<usize, error::socket_io::Error<Buffer::Error>> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.socket_read::<_, 1>(
                connection_generation,
                socket_generation,
                buf,
                &mut self.socket_2,
            ),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }
}
