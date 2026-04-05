//! Example using the GBA crate.

#![no_std]
#![no_main]

use core::{convert::Infallible, net::{Ipv4Addr, SocketAddrV4}};

use gba::prelude::*;
use gba_mobile::{
    Digit, Dns, Driver, Link, Socket, Timer,
    config::mobile_system_gb,
    socket::{self, NoSocket},
};

#[derive(Debug)]
struct RingBuffer {
    buffer: [u8; 512],
    head: usize,
    tail: usize,
    full: bool,
}

impl RingBuffer {
    const fn new() -> Self {
        Self {
            buffer: [0; 512],
            head: 0,
            tail: 0,
            full: false,
        }
    }

    fn is_empty(&self) -> bool {
        !self.full && (self.head == self.tail)
    }

    fn push(&mut self, byte: u8) -> bool {
        if self.full {
            false
        } else {
            self.buffer[self.head] = byte;
            self.head = (self.head + 1) % 512;

            if self.head == self.tail {
                self.full = true;
            }

            true
        }
    }

    fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            None
        } else {
            let byte = self.buffer[self.tail];
            self.tail = (self.tail + 1) % 512;
            self.full = false;

            Some(byte)
        }
    }
}

impl socket::Buffer for RingBuffer {
    type ReadError = Infallible;
    type WriteError = Infallible;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::ReadError> {
        let mut read = 0;
        for byte_slot in buf {
            if let Some(byte) = self.pop() {
                *byte_slot = byte;
                read += 1;
            } else {
                break;
            }
        }
        Ok(read)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError> {
        let mut written = 0;
        for &byte in buf {
            if self.push(byte) {
                written += 1;
            } else {
                break;
            }
        }
        Ok(written)
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

#[unsafe(link_section = ".ewram")]
static mut DRIVER: Driver<Socket<RingBuffer>, NoSocket, Dns<14>> = Driver::new(
    Timer::_0,
    Socket::new(RingBuffer::new()),
    NoSocket,
    Dns::new(),
);

// TODO: This function should probably be unsafe.
#[allow(static_mut_refs)]
fn with_driver<T, F>(f: F) -> T
where
    F: FnOnce(&mut Driver<Socket<RingBuffer>, NoSocket, Dns<14>>) -> T,
{
    let previous_ime = IME.read();
    IME.write(false);
    let result = f(unsafe { &mut DRIVER });
    IME.write(previous_ime);
    result
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    mgba_log::fatal!("the program crashed; see logs for panic info");
    loop {}
}

#[unsafe(link_section = ".iwram")]
extern "C" fn irq_handler(bits: IrqBits) {
    // To use gba_mobile, you must provide an interrupt handler that calls the library's interrupt
    // handler functions.
    if bits.vblank() {
        with_driver(Driver::vblank);
    }
    if bits.timer0() {
        with_driver(Driver::timer);
    }
    if bits.serial() {
        with_driver(Driver::serial);
    }
}

#[unsafe(no_mangle)]
pub fn main() {
    let _ = mgba_log::init();

    RUST_IRQ_HANDLER.write(Some(irq_handler));
    DISPSTAT.write(DisplayStatus::new().with_irq_vblank(true));
    IE.write(
        IrqBits::new()
            .with_vblank(true)
            .with_timer0(true)
            .with_serial(true),
    );
    IME.write(true);

    VBlankIntrWait();

    let pending_link = with_driver(Link::new);

    let status = loop {
        VBlankIntrWait();

        let status = with_driver(|driver| pending_link.status(driver));

        if let Ok(None) = status {
            continue;
        }
        break status;
    };

    log::info!("link connection status: {status:?}");

    if let Ok(Some(link)) = status {
        log::info!(
            "connected to {} adapter",
            with_driver(|driver| link.adapter(driver)).expect("unable to check adapter")
        );

        let write_config = mobile_system_gb::Config {
            registration: mobile_system_gb::Registration::Complete,
            primary_dns: Ipv4Addr::LOCALHOST,
            secondary_dns: Ipv4Addr::UNSPECIFIED,
            login_id: *b"test id   ",
            email: *b"fake_email@test.com     ",
            smtp_server: *b"abcdefghijklmnopqrst",
            pop_server: [0; 19],
            configuration_slots: Default::default(),
        };
        with_driver(|driver| link.write_config(driver, write_config))
            .expect("couldn't write config");

        let config: Result<mobile_system_gb::Config, _> = with_driver(|driver| link.config(driver));
        log::info!("attempted to parse Mobile System GB config: {config:?}");

        let pending_ppp = {
            log::info!("logging in!");
            with_driver(|driver| {
                link.login(
                    driver,
                    // #9677
                    [
                        Digit::try_from(b'#').unwrap(),
                        Digit::try_from(b'9').unwrap(),
                        Digit::try_from(b'6').unwrap(),
                        Digit::try_from(b'7').unwrap(),
                        Digit::try_from(b'7').unwrap(),
                    ]
                    .as_slice(),
                    [],
                    [],
                    Ipv4Addr::from_octets([8, 8, 8, 8]),
                    Ipv4Addr::from_octets([8, 8, 4, 4]),
                )
            })
            .expect("login failed")
        };
        let ppp_status = loop {
            VBlankIntrWait();

            let status = with_driver(|driver| pending_ppp.status(driver));

            if let Ok(None) = status {
                continue;
            }
            break status;
        };
        log::info!("ppp connection status: {ppp_status:?}");

        if let Ok(Some(ppp)) = ppp_status {
            let pending_dns = with_driver(|driver| ppp.dns(driver, "www.google.com"))
                .expect("DNS request failed");
            let dns_result = loop {
                VBlankIntrWait();

                let status = with_driver(|driver| pending_dns.status(driver)).expect("DNS failure");

                if let Some(result) = status {
                    break result;
                }
            };

            let pending_tcp = with_driver(|driver| ppp.socket_1_tcp(driver, SocketAddrV4::new(dns_result, 80)))
                .expect("TCP connection attempt failed");
            let tcp_status = loop {
                VBlankIntrWait();

                let status = with_driver(|driver| pending_tcp.status(driver));

                if let Ok(None) = status {
                    continue;
                }
                break status;
            };
            log::info!("tcp connection status: {tcp_status:?}");

            if let Ok(Some(mut tcp)) = tcp_status {
                const REQUEST: &'static [u8] = b"GET / HTTP/1.1\r\nHost: google.com\r\n\r\n";
                let mut request = REQUEST;
                loop {
                    VBlankIntrWait();
                    let amount_written =
                        with_driver(|driver| tcp.write(driver, &request).expect("write failed"));
                    request = &request[amount_written..];
                    if request.is_empty() {
                        log::info!("write finished");
                        break;
                    }
                }

                loop {
                    VBlankIntrWait();
                    let mut buffer = [0; 256];
                    let read_amount =
                        with_driver(|driver| tcp.read(driver, &mut buffer).expect("read failed"));
                    let s = str::from_utf8(&buffer[..read_amount]).expect("non-utf8 response");
                    if !s.is_empty() {
                        log::debug!("read amount: {read_amount}");
                        log::info!("data read: {s}");
                    }
                }
            }

            // In theory, UDP works. But libmobile is bugged to not return another packet on retry
            // in SIO32, and UDP not being implemented there means that this attempts to retry
            // receiving the packet.

            // let pending_udp = ppp
            //     .open_udp("www.example.com:80")
            //     .expect("UDP connection attempt failed");
            // let udp_status = loop {
            //     VBlankIntrWait();

            //     let status = pending_udp.status();

            //     if let Ok(None) = status {
            //         continue;
            //     }
            //     break status;
            // };
            // log::info!("udp connection status: {udp_status:?}");
        }

        let pending_p2p = loop {
            let keys = gba::mmio::KEYINPUT.read();
            if keys.a() {
                log::info!("connecting!");
                let pending_p2p = with_driver(|driver| link.connect(driver, Ipv4Addr::LOCALHOST))
                    .expect("p2p connection failed");
                break pending_p2p;
            } else if keys.b() {
                log::info!("accepting!");
                let pending_p2p =
                    with_driver(|driver| link.accept(driver)).expect("p2p connection failed");
                break pending_p2p;
            }
        };

        let p2p_status = loop {
            VBlankIntrWait();

            let status = with_driver(|driver| pending_p2p.status(driver));

            if let Ok(None) = status {
                continue;
            }
            break status;
        };

        log::info!("p2p connection status: {p2p_status:?}");
    }

    loop {
        VBlankIntrWait();
    }
}

#[unsafe(no_mangle)]
pub fn __sync_synchronize() {}
