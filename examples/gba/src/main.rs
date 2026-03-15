//! Example using the GBA crate.

#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use core::net::Ipv4Addr;

use gba::prelude::*;
use gba_mobile::Timer;

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
    if bits.serial() {
        gba_mobile::serial();
    }
    if bits.timer0() {
        gba_mobile::timer();
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
        log::info!("connected to {} adapter", link.adapter().expect("unable to check adapter"));
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
