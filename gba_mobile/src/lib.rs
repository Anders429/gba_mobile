#![no_std]
#![cfg_attr(test, no_main)]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(gba_test::runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_harness")]

#[cfg(test)]
extern crate alloc;

mod engine;
mod mmio;

#[cfg(test)]
#[unsafe(no_mangle)]
pub fn main() {
    let _ = mgba_log::init();
    test_harness()
}
