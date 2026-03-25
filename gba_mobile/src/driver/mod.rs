pub(crate) mod error;

mod active;
mod adapter;
mod command;
mod frames;
mod timers;

use core::net::Ipv4Addr;

pub use adapter::Adapter;

use crate::{
    ArrayVec, Config, Digit, Generation, Timer,
    mmio::{
        interrupt,
        serial::{self, RCNT, SIOCNT, TransferLength},
    },
    socket,
};
use active::Active;
use command::Command;
use either::Either;
use error::Error;

#[derive(Debug)]
enum State {
    /// Not currently linked with a Mobile Adapter device.
    Inactive,
    /// Currently linked with a Mobile Adapter device.
    Active(Active),
    /// Communication encountered an error and the link must be reset.
    Error(Error),
}

#[derive(Debug)]
pub(crate) struct Driver {
    link_generation: Generation,

    state: State,
}

impl Driver {
    pub(crate) const fn new() -> Self {
        Self {
            link_generation: Generation::new(),

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

    pub(crate) fn link(&mut self, timer: Timer) -> Generation {
        self.link_generation = self.link_generation.increment();
        Self::enable_interrupts(timer);
        match &mut self.state {
            State::Inactive | State::Error(_) => {
                Self::enable_communication();
                self.state = State::Active(Active::new(timer));
            }
            State::Active(active) => {
                active.start_link(timer);
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
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
    ) -> Result<bool, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.connection_status(connection_generation),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn disconnect(&mut self) {
        todo!()
    }

    /// Returns `Ok(None)` if there are no available sockets.
    pub(crate) fn open_tcp(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        host: Either<Ipv4Addr, ArrayVec<u8, 255>>,
        port: u16,
    ) -> Result<Option<(Generation, socket::Index)>, error::connection::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => active.open_tcp(connection_generation, host, port),
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
    }

    pub(crate) fn socket_status(
        &mut self,
        link_generation: Generation,
        connection_generation: Generation,
        socket_generation: Generation,
        index: socket::Index,
    ) -> Result<bool, error::socket::Error> {
        if self.link_generation != link_generation {
            return Err(error::link::Error::superseded().into());
        }

        match &mut self.state {
            State::Inactive => Err(error::link::Error::closed().into()),
            State::Active(active) => {
                active.socket_status(connection_generation, socket_generation, index)
            }
            State::Error(error) => Err(error::link::Error::from(error.clone()).into()),
        }
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

    pub(crate) fn vblank(&mut self) {
        match &mut self.state {
            State::Inactive => {}
            State::Active(active) => {
                if let Err(timeout) = active.vblank() {
                    self.state = State::Error(Error::Timeout(timeout));
                }
            }
            State::Error(_) => {}
        }
    }

    pub(crate) fn timer(&mut self) {
        match &mut self.state {
            State::Inactive => {}
            State::Active(active) => active.timer(),
            State::Error(_) => {}
        }
    }

    pub(crate) fn serial(&mut self) {
        match &mut self.state {
            State::Inactive => {}
            State::Active(active) => match active.serial() {
                Ok(active::StateChange::StillActive) => {}
                Ok(active::StateChange::Inactive) => self.state = State::Inactive,
                Err(error) => self.state = State::Error(Error::Error(error)),
            },
            State::Error(_) => {}
        }
    }
}
