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
pub mod pending;
pub mod socket;

mod arrayvec;
mod driver;
mod generation;
mod mmio;
mod timer;

#[doc(inline)]
pub use config::Config;
#[doc(inline)]
pub use connection::Connection;
#[doc(inline)]
pub use digit::Digit;
#[doc(inline)]
pub use dns::Dns;
pub use driver::{Adapter, Driver};
#[doc(inline)]
pub use internet::Internet;
#[doc(inline)]
pub use link::Link;
#[doc(inline)]
pub use pending::Pending;
#[doc(inline)]
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
