pub mod registration;
pub mod segments;

mod error;
mod phone_number;
mod slot;

use deranged::RangedU8;
pub use error::Error;
pub use phone_number::PhoneNumber;
pub use registration::Registration;
pub use slot::Slot;

use crate::config::format::Location;
use core::{net::Ipv4Addr, ptr};
use segments::Segments;

#[derive(Clone, Debug)]
pub struct Config {
    pub registration: Registration,
    pub primary_dns: Ipv4Addr,
    pub secondary_dns: Ipv4Addr,
    pub login_id: [u8; 10],
    pub email: [u8; 24],
    pub smtp_server: [u8; 20],
    pub pop_server: [u8; 19],
    pub configuration_slots: [Slot; 3],
}

impl Config {
    pub const fn new() -> Segments {
        Segments::Data
    }

    fn checksum(&self) -> u16 {
        fn array_checksum(bytes: &[u8]) -> u16 {
            bytes
                .iter()
                .copied()
                .fold(0u16, |sum, byte| sum.wrapping_add(byte as u16))
        }

        // Starting value account for the 'MA' header.
        let mut checksum: u16 = 0x8e;

        // Registration
        checksum = checksum.wrapping_add(self.registration as u16);

        // DNS.
        checksum = checksum.wrapping_add(array_checksum(self.primary_dns.octets().as_slice()));
        checksum = checksum.wrapping_add(array_checksum(self.secondary_dns.octets().as_slice()));

        // User data.
        checksum = checksum.wrapping_add(array_checksum(self.login_id.as_slice()));
        checksum = checksum.wrapping_add(array_checksum(self.email.as_slice()));

        // Servers
        checksum = checksum.wrapping_add(array_checksum(self.smtp_server.as_slice()));
        checksum = checksum.wrapping_add(array_checksum(self.pop_server.as_slice()));

        // Configuration slots.
        checksum = checksum.wrapping_add(array_checksum(
            self.configuration_slots[0]
                .phone_number
                .as_raw_bytes()
                .as_slice(),
        ));
        checksum = checksum.wrapping_add(array_checksum(self.configuration_slots[0].id.as_slice()));
        checksum = checksum.wrapping_add(array_checksum(
            self.configuration_slots[1]
                .phone_number
                .as_raw_bytes()
                .as_slice(),
        ));
        checksum = checksum.wrapping_add(array_checksum(self.configuration_slots[1].id.as_slice()));
        checksum = checksum.wrapping_add(array_checksum(
            self.configuration_slots[2]
                .phone_number
                .as_raw_bytes()
                .as_slice(),
        ));
        checksum = checksum.wrapping_add(array_checksum(self.configuration_slots[2].id.as_slice()));

        checksum
    }
}

impl super::Format for Config {
    const WRITES: usize = 2;

    type Segments = Segments;
    type Error = Error;

    fn segments() -> Self::Segments {
        Segments::Data
    }

    fn write(&self, request: usize, bytes: &mut [u8; 128]) -> Location {
        match request {
            0 => {
                // Header.
                bytes[0] = b'M';
                bytes[1] = b'A';

                // Registration.
                bytes[2] = self.registration as u8;

                // DNS.
                unsafe {
                    ptr::copy_nonoverlapping(
                        self.primary_dns.octets().as_ptr(),
                        bytes.as_mut_ptr().add(4),
                        4,
                    );
                    ptr::copy_nonoverlapping(
                        self.secondary_dns.octets().as_ptr(),
                        bytes.as_mut_ptr().add(8),
                        4,
                    );
                }

                // User data.
                unsafe {
                    ptr::copy_nonoverlapping(
                        self.login_id.as_ptr(),
                        bytes.as_mut_ptr().add(12),
                        10,
                    );
                    ptr::copy_nonoverlapping(self.email.as_ptr(), bytes.as_mut_ptr().add(44), 24);
                }

                // Servers.
                unsafe {
                    ptr::copy_nonoverlapping(
                        self.smtp_server.as_ptr(),
                        bytes.as_mut_ptr().add(74),
                        20,
                    );
                    ptr::copy_nonoverlapping(
                        self.pop_server.as_ptr(),
                        bytes.as_mut_ptr().add(94),
                        19,
                    );
                }

                Location {
                    offset: 0,
                    length: RangedU8::new_static::<0x71>(),
                }
            }
            1 => {
                // Configuration slots.
                unsafe {
                    ptr::copy_nonoverlapping(
                        self.configuration_slots[0]
                            .phone_number
                            .as_raw_bytes()
                            .as_ptr(),
                        bytes.as_mut_ptr().add(5),
                        8,
                    );
                    ptr::copy_nonoverlapping(
                        self.configuration_slots[0].id.as_ptr(),
                        bytes.as_mut_ptr().add(13),
                        16,
                    );
                    ptr::copy_nonoverlapping(
                        self.configuration_slots[1]
                            .phone_number
                            .as_raw_bytes()
                            .as_ptr(),
                        bytes.as_mut_ptr().add(29),
                        8,
                    );
                    ptr::copy_nonoverlapping(
                        self.configuration_slots[1].id.as_ptr(),
                        bytes.as_mut_ptr().add(37),
                        16,
                    );
                    ptr::copy_nonoverlapping(
                        self.configuration_slots[2]
                            .phone_number
                            .as_raw_bytes()
                            .as_ptr(),
                        bytes.as_mut_ptr().add(53),
                        8,
                    );
                    ptr::copy_nonoverlapping(
                        self.configuration_slots[2].id.as_ptr(),
                        bytes.as_mut_ptr().add(61),
                        16,
                    );
                }

                let checksum = self.checksum();
                bytes[77] = (checksum >> 8) as u8;
                bytes[78] = checksum as u8;

                Location {
                    offset: 0x71,
                    length: RangedU8::new_static::<0x4f>(),
                }
            }
            _ => unreachable!(),
        }
    }
}
