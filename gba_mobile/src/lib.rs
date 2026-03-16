#![no_std]
#![cfg_attr(test, no_main)]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(gba_test::runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_harness")]
#![allow(static_mut_refs)]

#[cfg(test)]
extern crate alloc;

pub mod config;
pub mod digit;
pub mod link;
pub mod p2p;

mod arrayvec;
mod driver;
mod generation;
mod mmio;
mod timer;

pub use config::Config;
pub use digit::Digit;
pub use driver::Adapter;
pub use link::Link;
pub use timer::Timer;

use arrayvec::ArrayVec;
use driver::Driver;
use generation::Generation;
use mmio::interrupt;

#[unsafe(link_section = ".ewram")]
static mut DRIVER: Driver = Driver::new();

pub fn vblank() {
    unsafe {
        let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
        interrupt::MASTER_ENABLE.write_volatile(false);
        DRIVER.vblank();
        interrupt::MASTER_ENABLE.write_volatile(prev_enable);
    }
}

pub fn timer() {
    unsafe {
        let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
        interrupt::MASTER_ENABLE.write_volatile(false);
        DRIVER.timer();
        interrupt::MASTER_ENABLE.write_volatile(prev_enable);
    }
}

pub fn serial() {
    unsafe {
        let prev_enable = interrupt::MASTER_ENABLE.read_volatile();
        interrupt::MASTER_ENABLE.write_volatile(false);
        DRIVER.serial();
        interrupt::MASTER_ENABLE.write_volatile(prev_enable);
    }
}

#[cfg(test)]
#[unsafe(no_mangle)]
pub fn main() {
    let _ = mgba_log::init();
    test_harness()
}
