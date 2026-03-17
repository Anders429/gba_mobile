pub mod registration;

mod error;
mod phone_number;
mod slot;

use core::{net::Ipv4Addr, ptr};

use deranged::RangedUsize;
pub use error::Error;
pub use phone_number::PhoneNumber;
pub use registration::Registration;
pub use slot::Slot;

#[derive(Debug)]
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

impl super::Config for Config {
    type Error = Error;

    fn read(bytes: &[u8; 256]) -> Result<Self, Self::Error> {
        // Header.
        if bytes[0] != b'M' {
            return Err(Error::HeaderM(bytes[0]));
        }
        if bytes[1] != b'A' {
            return Err(Error::HeaderA(bytes[1]));
        }

        // Registration.
        let registration = Registration::try_from(bytes[2])?;

        // DNS.
        let primary_dns = Ipv4Addr::from_octets([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let secondary_dns = Ipv4Addr::from_octets([bytes[8], bytes[9], bytes[10], bytes[11]]);

        // User data.
        let mut login_id = [0; 10];
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().add(12), login_id.as_mut_ptr(), 10);
        }
        let mut email = [0; 24];
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().add(44), email.as_mut_ptr(), 24);
        }

        // Servers.
        let mut smtp_server = [0; 20];
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().add(74), smtp_server.as_mut_ptr(), 20);
        }
        let mut pop_server = [0; 19];
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().add(94), pop_server.as_mut_ptr(), 19);
        }

        // Configuration slots.
        fn read_configuration_slot(bytes: &[u8; 256], index: RangedUsize<0, 2>) -> Slot {
            let mut phone_number_raw = [0; 8];
            unsafe {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr().add(118 + (24 * index.get())),
                    phone_number_raw.as_mut_ptr(),
                    8,
                );
            }
            let phone_number = PhoneNumber::from_raw_bytes(phone_number_raw);

            let mut id = [0; 16];
            unsafe {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr().add(126 + (24 * index.get())),
                    id.as_mut_ptr(),
                    16,
                );
            }

            Slot { phone_number, id }
        }
        let configuration_slots = [
            read_configuration_slot(bytes, RangedUsize::new_static::<0>()),
            read_configuration_slot(bytes, RangedUsize::new_static::<1>()),
            read_configuration_slot(bytes, RangedUsize::new_static::<2>()),
        ];

        // Checksum.
        let calculated_checksum = bytes[..190]
            .iter()
            .copied()
            .fold(0u16, |sum, byte| sum.wrapping_add(byte as u16));
        let received_checksum = ((bytes[190] as u16) << 8) | (bytes[191] as u16);
        if calculated_checksum != received_checksum {
            return Err(Error::Checksum {
                calculated: calculated_checksum,
                received: received_checksum,
            });
        }

        Ok(Self {
            registration,
            primary_dns,
            secondary_dns,
            login_id,
            email,
            smtp_server,
            pop_server,
            configuration_slots,
        })
    }

    fn write(&self, bytes: &mut [u8; 256]) {
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
            ptr::copy_nonoverlapping(self.login_id.as_ptr(), bytes.as_mut_ptr().add(12), 10);
            ptr::copy_nonoverlapping(self.email.as_ptr(), bytes.as_mut_ptr().add(44), 24);
        }

        // Servers.
        unsafe {
            ptr::copy_nonoverlapping(self.smtp_server.as_ptr(), bytes.as_mut_ptr().add(74), 20);
            ptr::copy_nonoverlapping(self.pop_server.as_ptr(), bytes.as_mut_ptr().add(94), 19);
        }

        // Configuration slots.
        unsafe {
            ptr::copy_nonoverlapping(
                self.configuration_slots[0]
                    .phone_number
                    .as_raw_bytes()
                    .as_ptr(),
                bytes.as_mut_ptr().add(118),
                8,
            );
            ptr::copy_nonoverlapping(
                self.configuration_slots[0].id.as_ptr(),
                bytes.as_mut_ptr().add(126),
                16,
            );
            ptr::copy_nonoverlapping(
                self.configuration_slots[1]
                    .phone_number
                    .as_raw_bytes()
                    .as_ptr(),
                bytes.as_mut_ptr().add(142),
                8,
            );
            ptr::copy_nonoverlapping(
                self.configuration_slots[1].id.as_ptr(),
                bytes.as_mut_ptr().add(150),
                16,
            );
            ptr::copy_nonoverlapping(
                self.configuration_slots[2]
                    .phone_number
                    .as_raw_bytes()
                    .as_ptr(),
                bytes.as_mut_ptr().add(166),
                8,
            );
            ptr::copy_nonoverlapping(
                self.configuration_slots[2].id.as_ptr(),
                bytes.as_mut_ptr().add(174),
                16,
            );
        }

        // Checksum.
        let checksum = bytes[..190]
            .iter()
            .copied()
            .fold(0u16, |sum, byte| sum.wrapping_add(byte as u16));
        bytes[190] = (checksum >> 8) as u8;
        bytes[191] = checksum as u8;
    }
}
