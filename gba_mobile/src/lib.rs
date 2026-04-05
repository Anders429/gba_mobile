#![no_std]
#![cfg_attr(test, no_main)]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(gba_test::runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_harness")]

#[cfg(test)]
extern crate alloc;

pub mod config;
pub mod connection;
pub mod digit;
pub mod dns;
pub mod internet;
pub mod link;
pub mod socket;

mod arrayvec;
mod driver;
mod generation;
mod mmio;
mod timer;

pub use config::Config;
pub use digit::Digit;
pub use dns::Dns;
pub use driver::{Adapter, Driver};
pub use link::Link;
pub use socket::Socket;
pub use timer::Timer;

use arrayvec::ArrayVec;
use generation::Generation;

#[cfg(test)]
#[unsafe(no_mangle)]
pub fn main() {
    let _ = mgba_log::init();
    test_harness()
}
