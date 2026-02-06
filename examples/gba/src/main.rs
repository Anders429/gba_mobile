//! Example using the GBA crate.

#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use gba::prelude::*;
use gba_mobile::Timer;

#[unsafe(link_section = ".iwram")]
static mut MOBILE_ENGINE: gba_mobile::Engine = gba_mobile::Engine::new(Timer::_0);

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    mgba_log::fatal!("the program crashed; see logs for panic info");
    loop {}
}

#[unsafe(link_section = ".iwram")]
extern "C" fn irq_handler(bits: IrqBits) {
    // To use gba_mobile, you must provide an interrupt handler that calls gba_mobile's interrupt
    // handler functions.
    if bits.vblank() {
        unsafe {
            MOBILE_ENGINE.vblank();
        }
    }
    if bits.timer0() {
        unsafe {
            MOBILE_ENGINE.timer();
        }
    }
    if bits.serial() {
        unsafe {
            MOBILE_ENGINE.serial();
        }
    }
}

#[unsafe(no_mangle)]
pub fn main() {
    let _ = mgba_log::init();

    RUST_IRQ_HANDLER.write(Some(irq_handler));
    DISPSTAT.write(DisplayStatus::new().with_irq_vblank(true));
    IE.write(IrqBits::new().with_vblank(true).with_serial(true));
    IME.write(true);

    VBlankIntrWait();

    let pending_link = unsafe {
        IME.write(false);
        let pending_link = MOBILE_ENGINE.link_p2p();
        IME.write(true);
        pending_link
    };

    let status = loop {
        VBlankIntrWait();
        let status = unsafe {
            IME.write(false);
            let status = pending_link.status(&MOBILE_ENGINE);
            IME.write(true);
            status
        };
        if let Ok(None) = status {
            continue;
        }
        break status;
    };

    log::info!("link connection status: {status:?}")
}

#[unsafe(no_mangle)]
pub fn __sync_synchronize() {}
