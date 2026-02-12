//! Example using the GBA crate.

#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use gba::prelude::*;
use gba_mobile::Timer;

#[unsafe(link_section = ".ewram")]
static mut MOBILE_DRIVER: gba_mobile::Driver = gba_mobile::Driver::new(Timer::_0);

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    mgba_log::fatal!("the program crashed; see logs for panic info");
    loop {}
}

#[unsafe(link_section = ".iwram")]
extern "C" fn irq_handler(bits: IrqBits) {
    // To use gba_mobile, you must provide an interrupt handler that calls the driver's interrupt
    // handler functions.
    if bits.vblank() {
        unsafe {
            MOBILE_DRIVER.vblank();
        }
    }
    if bits.serial() {
        unsafe {
            MOBILE_DRIVER.serial();
        }
    }
    if bits.timer0() {
        unsafe {
            MOBILE_DRIVER.timer();
        }
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

    IME.write(false);
    let pending_link = unsafe { MOBILE_DRIVER.link() };
    IME.write(true);

    let status = loop {
        VBlankIntrWait();

        IME.write(false);
        let status = unsafe { pending_link.status(&MOBILE_DRIVER) };
        IME.write(true);

        if let Ok(None) = status {
            continue;
        }
        break status;
    };

    log::info!("link connection status: {status:?}");

    if let Ok(Some(mut link)) = status {
        IME.write(false);
        let pending_p2p = link
            .accept(unsafe { &mut MOBILE_DRIVER })
            .expect("p2p connection failed");
        IME.write(true);

        let p2p_status = loop {
            VBlankIntrWait();

            IME.write(false);
            let status = unsafe { pending_p2p.status(&MOBILE_DRIVER) };
            IME.write(true);

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
