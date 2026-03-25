//! Example using the GBA crate.

#![no_std]
#![no_main]

use core::net::Ipv4Addr;

use gba::prelude::*;
use gba_mobile::{Digit, Timer, config::mobile_system_gb};

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
        gba_mobile::vblank();
    }
    if bits.timer0() {
        gba_mobile::timer();
    }
    if bits.serial() {
        gba_mobile::serial();
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

    let pending_link = gba_mobile::Link::new(Timer::_0);

    let status = loop {
        VBlankIntrWait();

        let status = pending_link.status();

        if let Ok(None) = status {
            continue;
        }
        break status;
    };

    log::info!("link connection status: {status:?}");

    if let Ok(Some(link)) = status {
        log::info!(
            "connected to {} adapter",
            link.adapter().expect("unable to check adapter")
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
        link.write_config(write_config)
            .expect("couldn't write config");

        let config = link.config::<mobile_system_gb::Config>();
        log::info!("attempted to parse Mobile System GB config: {config:?}");

        let pending_ppp = {
            log::info!("logging in!");
            link.login(
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
            .expect("login failed")
        };
        let ppp_status = loop {
            VBlankIntrWait();

            let status = pending_ppp.status();

            if let Ok(None) = status {
                continue;
            }
            break status;
        };
        log::info!("ppp connection status: {ppp_status:?}");

        if let Ok(Some(ppp)) = ppp_status {
            let pending_tcp = ppp
                .open_tcp("www.google.com:80")
                .expect("TCP connection attempt failed");
            let tcp_status = loop {
                VBlankIntrWait();

                let status = pending_tcp.status();

                if let Ok(None) = status {
                    continue;
                }
                break status;
            };
            log::info!("tcp connection status: {tcp_status:?}");
        }

        let pending_p2p = loop {
            let keys = gba::mmio::KEYINPUT.read();
            if keys.a() {
                log::info!("connecting!");
                let pending_p2p = link
                    .connect(Ipv4Addr::LOCALHOST)
                    .expect("p2p connection failed");
                break pending_p2p;
            } else if keys.b() {
                log::info!("accepting!");
                let pending_p2p = link.accept().expect("p2p connection failed");
                break pending_p2p;
            }
        };

        let p2p_status = loop {
            VBlankIntrWait();

            let status = pending_p2p.status();

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
