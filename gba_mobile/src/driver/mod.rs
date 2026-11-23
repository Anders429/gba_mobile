pub(crate) mod active;
pub(crate) mod error;

mod adapter;
mod command;
mod frames;
mod timers;

pub use adapter::Adapter;

use crate::{
    ArrayVec, Config, Digit, Dns, Generation, Socket, Timer, config, dns,
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
enum State<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    /// Not currently linked with a Mobile Adapter device.
    Inactive,
    /// Currently linked with a Mobile Adapter device.
    Active(Active<Socket1, Socket2, Dns, Config>),
    /// Communication encountered an error and the link must be reset.
    Error(Error<Socket1, Socket2, Dns, Config>),
}

/// Driver for connecting with a Mobile Adapter device.
///
/// The `Driver` handles establishing a connection with a Mobile Adapter, sending and receiving
/// data packets, and handling communication errors. The design is intended to abstract away the
/// asynchronous communication from your code, only requiring you to worry about issuing high-level
/// commands and awaiting their results.
///
/// ## Customization
///
/// There are multiple operations a `Driver` is capable of that require additional nontrivial
/// memory usage. In order to not require you to pay for what you don't use, these components must
/// be configured on construction of a `Driver`.
///
/// The customizable components are:
/// - Sockets (up to two may be configured).
/// - DNS.
/// - Configuration.
///
/// ### Sockets
///
/// The Mobile Adapter `Driver` supports using up to two separate sockets for sending and receiving
/// data over a connection. In order to use a socket, you must provide a [`Socket`] to give the
/// `Driver` buffers to read and write data.
///
/// Which sockets to configure depends on what you want to use the `Driver` for:
/// - Socket 1: Used for peer-to-peer connections. Can be used for TCP/UDP connections.
/// - Socket 2: Can be used for TCP/UDP connections.
///
/// To send and receive data over a socket, provide a `Socket<Buffer>`. See [`socket`] for more
/// information on providing a `Buffer`.
///
/// ### DNS
///
/// If you wish to perform DNS requests when connected to the internet, you must use [`Dns`],
/// specifying the max buffer size. For example, use `Dns<64>` to allow lookup requests for domains
/// up to 64 bytes long. If you do not wish to perform DNS requests, simply use [`NoDns`].
///
/// ### Configuration
///
/// To interact with the Mobile Adapter's on-device EEPROM storage, a configuration format must be
/// provided. You'll most likely want to use the [`mobile_system_gb::Config`] provided by this
/// crate, which implements the official configuration format used by all Mobile System GB games.
///
/// To use a config format, provide it as a type parameter to the [`Config`] type. If you do not
/// wish to interact with the Mobile Adapter's configuration, simply use [`NoConfig`].
///
/// For more information on Mobile Adapter configuration formats, see [`config`].
///
/// ## Interrupts
///
/// As the packets being sent are larger than 4 bytes, the communication state cannot progress from
/// within a single function call without blocking. Instead, the state progresses asynchronously,
/// powered by interrupts. Therefore, to make a `Driver` work, you must hook up the following
/// functions to be called on their respective interrupts:
///
/// - [`vblank()`] - For keeping idle connections alive and handling timeouts.
/// - [`serial()`] - For handling incoming serial communications from the Mobile Adapter.
/// - [`timer()`] - For handling outgoing serial communications to the Mobile Adapter. This should
/// be called when the interrupt for the [`Timer`] provided on `Driver` creation fires.
///
/// These interrupts are how the asynchronous functions progress. Without proper setup, the
/// `Driver` will be unable to function.
///
/// ### Safety
///
/// Each of the interrupt functions require a `&mut Driver`. The functions you'll be calling from
/// non-interrupt code also require a `&mut Driver`. In order to do this safely, you **must**
/// guarantee that an interrupt will not be handled while a mutable reference to your driver
/// already exists.
///
/// A common pattern is to use a `static mut` and disable `IME` while you hold any mutable
/// reference.
///
/// ### Example Setup
///
/// ``` rust
/// use core::ops::FnOnce;
/// use gba_mobile::{
///     Timer,
///     config::NoConfig,
///     dns::NoDns,
///     socket::NoSocket,
/// };
///
/// /// This is a bare-bones `Driver` configuration.
/// type Driver = gba_mobile::Driver<NoSocket, NoSocket, NoDns, NoConfig>;
///
/// #[unsafe(link_section = ".ewram")]
/// static mut DRIVER: Driver = Driver::new(
///     Timer::_3,
///     NoSocket,
///     NoSocket,
///     NoDns,
///     NoConfig,
/// );
///
/// const IME: *mut bool = 0x0400_0208 as *mut bool;
///
/// /// An example function that can be used to safely interact with a `&mut Driver`.
/// ///
/// /// It is still marked `unsafe` because the caller must guarantee there is no other mutable
/// /// reference to the static driver.
/// #[allow(static_mut_refs)]
/// unsafe fn with_driver<T, F>(f: F) -> T
/// where
///     F: FnOnce(
///         &mut Driver,
///     ) -> T,
/// {
///     unsafe {
///         let previous_ime = IME.read_volatile();
///         IME.write_volatile(false);
///         let result = f(&mut DRIVER);
///         IME.write_volatile(previous_ime);
///         result
///     }
/// }
///
/// /// An example interrupt handler.
/// ///
/// /// This is intended to be a framework-agnostic example. The way you actually set this up will
/// /// be dependent on the framework you are using. Consult the documentation for whatever
/// /// framework you are using as needed.
/// #[unsafe(link_section = ".iwram")]
/// extern "C" fn irq_handler(bits: u16) {
///     const VBLANK: u16 = 1;
///     const SERIAL: u16 = 128;
///     const TIMER_3: u16 = 64;
///
///     if bits & VBLANK != 0 {
///         unsafe {with_driver(Driver::vblank);}
///     }
///     if bits & SERIAL != 0 {
///         unsafe {with_driver(Driver::serial);}
///     }
///     if bits & TIMER_3 != 0 {
///         unsafe {with_driver(Driver::timer);}
///     }
/// }
/// ```
///
/// ## Usage
/// With a `Driver` that is configured and hooked up in your interrupt handler, you can connect to
/// the Mobile Adapter using [`Link::new()`]. This will return a [`Pending`] task that can be
/// polled as needed.
///
/// Assuming the above example setup, you can connect like this:
///
/// ```rust
/// let pending = unsafe {with_driver(Link::new)};
/// loop {
///     match unsafe {with_driver(|driver| pending.status(driver))} {
///         None => continue,
///         Some(Ok(link)) => {
///             // Do something with the link...
///             break;
///         },
///         Some(Err(error)) => {
///             // Driver error...
///             break;
///         }
///     }
/// }
/// ```
///
/// [`Link::new()`]: `crate::Link::new()`
/// [`mobile_system_gb::Config`]: `crate::config::mobile_system_gb::Config`
/// [`NoDns`]: `crate::dns::NoDns`
/// [`NoConfig`]: `crate::config::NoConfig`
/// [`Pending`]: crate::Pending
/// [`serial()`]: `Driver::serial()`
/// [`timer()`]: `Driver::timer()`
/// [`vblank()`]: `Driver::vblank()`
#[derive(Debug)]
pub struct Driver<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    link_generation: Generation,
    timer: Timer,

    socket_1: Socket1,
    socket_2: Socket2,
    dns: Dns,
    config: Config,

    state: State<Socket1, Socket2, Dns, Config>,
}

impl<Socket1, Socket2, Dns, Config> Driver<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub const fn new(
        timer: Timer,
        socket_1: Socket1,
        socket_2: Socket2,
        dns: Dns,
        config: Config,
    ) -> Self {
        Self {
            link_generation: Generation::new(),
            timer,

            socket_1,
            socket_2,
            dns,
            config,

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
    ) -> Result<
        ActiveDriver<'a, Socket1, Socket2, Dns, Config>,
        error::link::Error<Socket1, Socket2, Dns, Config>,
    > {
        if link_generation == self.link_generation {
            match &self.state {
                State::Inactive => Err(error::link::Error::closed()),
                State::Active(active) => Ok(ActiveDriver {
                    socket_1: &self.socket_1,
                    socket_2: &self.socket_2,
                    dns: &self.dns,
                    config: &self.config,

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
    ) -> Result<
        ActiveDriverMut<'a, Socket1, Socket2, Dns, Config>,
        error::link::Error<Socket1, Socket2, Dns, Config>,
    > {
        if link_generation == self.link_generation {
            match &mut self.state {
                State::Inactive => Err(error::link::Error::closed()),
                State::Active(active) => Ok(ActiveDriverMut {
                    socket_1: &mut self.socket_1,
                    socket_2: &mut self.socket_2,
                    dns: &mut self.dns,
                    config: &mut self.config,

                    active,
                }),
                State::Error(error) => Err(error.clone().into()),
            }
        } else {
            Err(error::link::Error::superseded())
        }
    }

    /// Timer interrupt handler.
    ///
    /// This should be called whenever a timer interrupt is received for the timer the driver was
    /// constructed with.
    pub fn timer(&mut self) {
        match &mut self.state {
            State::Inactive => {}
            State::Active(active) => active.timer(self.timer),
            State::Error(_) => {}
        }
    }

    /// Serial interrupt handler.
    ///
    /// This should be called whenever a serial interrupt is received.
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
                    &mut self.config,
                ) {
                    self.state = State::Error(Error::Error(error));
                }
            }
            State::Error(_) => {}
        }
    }

    /// V-Blank interrupt handler.
    ///
    /// This should be called whenever a V-Blank interrupt is received.
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
                    &self.config,
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
pub(crate) struct ActiveDriver<'a, Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    socket_1: &'a Socket1,
    socket_2: &'a Socket2,
    dns: &'a Dns,
    config: &'a Config,

    active: &'a Active<Socket1, Socket2, Dns, Config>,
}

impl<'a, Socket1, Socket2, Dns, Config> ActiveDriver<'a, Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub(crate) fn link_status(
        self,
    ) -> Result<bool, error::link::Error<Socket1, Socket2, Dns, Config>> {
        self.active.link_status()
    }

    pub(crate) fn connection_status(
        self,
        connection_generation: Generation,
    ) -> Result<bool, error::connection::Error<Socket1, Socket2, Dns, Config>> {
        self.active.connection_status(connection_generation)
    }

    pub(crate) fn adapter(
        self,
    ) -> Result<Adapter, error::link::Error<Socket1, Socket2, Dns, Config>> {
        self.active.adapter()
    }

    pub(crate) fn ip(
        self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error<Socket1, Socket2, Dns, Config>> {
        self.active.ip(connection_generation)
    }

    pub(crate) fn primary_dns(
        self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error<Socket1, Socket2, Dns, Config>> {
        self.active.primary_dns(connection_generation)
    }

    pub(crate) fn secondary_dns(
        self,
        connection_generation: Generation,
    ) -> Result<Ipv4Addr, error::connection::Error<Socket1, Socket2, Dns, Config>> {
        self.active.secondary_dns(connection_generation)
    }
}

impl<'a, Buffer, Socket2, Dns, Config> ActiveDriver<'a, Socket<Buffer>, Socket2, Dns, Config>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub(crate) fn socket_1_status(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<bool, error::socket::Error<Socket<Buffer>, Socket2, Dns, Config>> {
        self.active
            .socket_status::<_, 0>(connection_generation, socket_generation, self.socket_1)
    }
}

impl<'a, Buffer, Socket1, Dns, Config> ActiveDriver<'a, Socket1, Socket<Buffer>, Dns, Config>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub(crate) fn socket_2_status(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Result<bool, error::socket::Error<Socket1, Socket<Buffer>, Dns, Config>> {
        self.active
            .socket_status::<_, 1>(connection_generation, socket_generation, &self.socket_2)
    }
}

impl<'a, Socket1, Socket2, Config, const MAX_LEN: usize>
    ActiveDriver<'a, Socket1, Socket2, Dns<MAX_LEN>, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Config: config::Mode,
{
    pub(crate) fn dns_status(
        self,
        connection_generation: Generation,
        dns_generation: Generation,
    ) -> Result<Option<Ipv4Addr>, error::dns::Error<Socket1, Socket2, Dns<MAX_LEN>, Config>> {
        self.active
            .dns_status(connection_generation, dns_generation, &self.dns)
    }
}

impl<'a, Socket1, Socket2, Dns, Format> ActiveDriver<'a, Socket1, Socket2, Dns, Config<Format>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Format: config::Format,
{
    pub(crate) fn config(
        self,
    ) -> Result<
        Result<Format, Format::Error>,
        error::config::Error<Socket1, Socket2, Dns, Config<Format>>,
    > {
        unsafe { self.active.config(self.config) }
    }
}

#[derive(Debug)]
pub(crate) struct ActiveDriverMut<'a, Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    socket_1: &'a mut Socket1,
    socket_2: &'a mut Socket2,
    dns: &'a mut Dns,
    config: &'a mut Config,

    active: &'a mut Active<Socket1, Socket2, Dns, Config>,
}

impl<'a, Socket1, Socket2, Dns, Config> ActiveDriverMut<'a, Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub(crate) fn close_link(
        self,
    ) -> Result<(), error::link::Error<Socket1, Socket2, Dns, Config>> {
        self.active.close_link()
    }

    pub(crate) fn login(
        &mut self,
        phone_number: ArrayVec<Digit, 32>,
        id: ArrayVec<u8, 32>,
        password: ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
    ) -> Result<Generation, error::link::Error<Socket1, Socket2, Dns, Config>> {
        self.active
            .login(phone_number, id, password, primary_dns, secondary_dns)
    }

    pub(crate) fn disconnect(
        self,
        connection_generation: Generation,
    ) -> Result<(), error::connection::Error<Socket1, Socket2, Dns, Config>> {
        self.active.disconnect(connection_generation)
    }
}

impl<'a, Buffer, Socket2, Dns, Config> ActiveDriverMut<'a, Socket<Buffer>, Socket2, Dns, Config>
where
    Buffer: socket::Buffer,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub(crate) fn accept(
        self,
    ) -> Result<Generation, error::link::Error<Socket<Buffer>, Socket2, Dns, Config>> {
        self.active.accept()
    }

    pub(crate) fn connect(
        self,
        phone_number: ArrayVec<Digit, 32>,
    ) -> Result<Generation, error::link::Error<Socket<Buffer>, Socket2, Dns, Config>> {
        self.active.connect(phone_number)
    }

    pub(crate) fn open_tcp_1(
        self,
        connection_generation: Generation,
        socket_addr: SocketAddrV4,
    ) -> Result<Generation, error::connection::Error<Socket<Buffer>, Socket2, Dns, Config>> {
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
    ) -> Result<Generation, error::connection::Error<Socket<Buffer>, Socket2, Dns, Config>> {
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
    ) -> Result<(), error::socket::Error<Socket<Buffer>, Socket2, Dns, Config>> {
        self.active
            .close_socket::<_, 0>(connection_generation, socket_generation, self.socket_1)
    }

    pub(crate) fn connection_read(
        self,
        connection_generation: Generation,
        buf: &mut [u8],
    ) -> Result<
        usize,
        error::connection_io::Error<Buffer::ReadError, Socket<Buffer>, Socket2, Dns, Config>,
    > {
        self.active
            .connection_read(connection_generation, buf, self.socket_1)
    }

    pub(crate) fn connection_write(
        self,
        connection_generation: Generation,
        buf: &[u8],
    ) -> Result<usize, error::connection::Error<Socket<Buffer>, Socket2, Dns, Config>> {
        self.active
            .connection_write(connection_generation, buf, self.socket_1)
    }

    pub(crate) fn connection_flush(
        self,
        connection_generation: Generation,
    ) -> Result<(), error::connection::Error<Socket<Buffer>, Socket2, Dns, Config>> {
        self.active
            .connection_flush(connection_generation, self.socket_1)
    }

    pub(crate) fn socket_1_read(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &mut [u8],
    ) -> Result<
        usize,
        error::socket_io::Error<Buffer::ReadError, Socket<Buffer>, Socket2, Dns, Config>,
    > {
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
    ) -> Result<usize, error::socket::Error<Socket<Buffer>, Socket2, Dns, Config>> {
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
    ) -> Result<(), error::socket::Error<Socket<Buffer>, Socket2, Dns, Config>> {
        self.active
            .socket_flush::<_, 0>(connection_generation, socket_generation, self.socket_1)
    }
}

impl<'a, Buffer, Socket1, Dns, Config> ActiveDriverMut<'a, Socket1, Socket<Buffer>, Dns, Config>
where
    Buffer: socket::Buffer,
    Socket1: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    pub(crate) fn open_tcp_2(
        self,
        connection_generation: Generation,
        socket_addr: SocketAddrV4,
    ) -> Result<Generation, error::connection::Error<Socket1, Socket<Buffer>, Dns, Config>> {
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
    ) -> Result<Generation, error::connection::Error<Socket1, Socket<Buffer>, Dns, Config>> {
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
    ) -> Result<(), error::socket::Error<Socket1, Socket<Buffer>, Dns, Config>> {
        self.active
            .close_socket::<_, 1>(connection_generation, socket_generation, self.socket_2)
    }

    pub(crate) fn socket_2_read(
        self,
        connection_generation: Generation,
        socket_generation: Generation,
        buf: &mut [u8],
    ) -> Result<
        usize,
        error::socket_io::Error<Buffer::ReadError, Socket1, Socket<Buffer>, Dns, Config>,
    > {
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
    ) -> Result<usize, error::socket::Error<Socket1, Socket<Buffer>, Dns, Config>> {
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
    ) -> Result<(), error::socket::Error<Socket1, Socket<Buffer>, Dns, Config>> {
        self.active
            .socket_flush::<_, 1>(connection_generation, socket_generation, self.socket_2)
    }
}

impl<'a, Socket1, Socket2, Config, const MAX_LEN: usize>
    ActiveDriverMut<'a, Socket1, Socket2, Dns<MAX_LEN>, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Config: config::Mode,
{
    pub(crate) fn dns(
        self,
        connection_generation: Generation,
        name: ArrayVec<u8, MAX_LEN>,
    ) -> Result<Generation, error::connection::Error<Socket1, Socket2, Dns<MAX_LEN>, Config>> {
        self.active.dns(connection_generation, name, self.dns)
    }

    pub(crate) fn cancel_dns(
        self,
        connection_generation: Generation,
        dns_generation: Generation,
    ) -> Result<(), error::dns::Error<Socket1, Socket2, Dns<MAX_LEN>, Config>> {
        self.active
            .cancel_dns(connection_generation, dns_generation, self.dns)
    }
}

impl<'a, Socket1, Socket2, Dns, Format> ActiveDriverMut<'a, Socket1, Socket2, Dns, Config<Format>>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Format: config::Format,
{
    pub(crate) fn write_config(
        &mut self,
        format: Format,
    ) -> Result<(), error::link::Error<Socket1, Socket2, Dns, Config<Format>>> {
        self.active.write_config(self.config, format)
    }
}
