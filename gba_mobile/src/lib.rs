#![no_std]
#![cfg_attr(test, no_main)]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(gba_test::runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_harness")]

#[cfg(test)]
extern crate alloc;

pub mod link_p2p;

mod engine;
mod mmio;
mod timer;

pub use engine::Engine;
pub use timer::Timer;

#[cfg(test)]
#[unsafe(no_mangle)]
pub fn main() {
    let _ = mgba_log::init();
    test_harness()
}
