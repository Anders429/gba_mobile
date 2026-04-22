use super::{Config, Error, PhoneNumber, Registration, Slot};
use crate::config::{
    Format, format,
    format::{Location, ReadResult},
};
use core::{net::Ipv4Addr, ptr};
use deranged::{RangedU8, RangedUsize};

#[derive(Debug)]
pub struct Data {
    registration: Registration,
    primary_dns: Ipv4Addr,
    secondary_dns: Ipv4Addr,
    login_id: [u8; 10],
    email: [u8; 24],
    smtp_server: [u8; 20],
    pop_server: [u8; 19],
}

#[derive(Debug)]
pub enum Segments {
    Data,
    ConfigSlots {
        data: Data,
        // This checksum isn't just derived from the fields we read; it's a sum of all bytes we
        // have read so far.
        checksum: u16,
    },
}

impl Segments {
    fn read_configuration_slot(bytes: &[u8; 128], index: RangedUsize<0, 2>) -> Slot {
        let mut phone_number_raw = [0; 8];
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr().add(5 + (24 * index.get())),
                phone_number_raw.as_mut_ptr(),
                8,
            );
        }
        let phone_number = PhoneNumber::from_raw_bytes(phone_number_raw);

        let mut id = [0; 16];
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr().add(13 + (24 * index.get())),
                id.as_mut_ptr(),
                16,
            );
        }

        Slot { phone_number, id }
    }
}

impl format::Segments for Segments {
    type Format = Config;

    fn location(&self) -> Location {
        match self {
            Self::Data => Location {
                offset: 0,
                length: RangedU8::new_static::<0x71>(),
            },
            Self::ConfigSlots { .. } => Location {
                offset: 0x71,
                length: RangedU8::new_static::<0x4f>(),
            },
        }
    }

    fn read(
        self,
        bytes: &[u8; 128],
    ) -> Result<ReadResult<Self::Format, Self>, <Self::Format as Format>::Error> {
        match self {
            Self::Data => {
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
                let secondary_dns =
                    Ipv4Addr::from_octets([bytes[8], bytes[9], bytes[10], bytes[11]]);

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

                // Checksum.
                let checksum = bytes[..0x71]
                    .iter()
                    .copied()
                    .fold(0u16, |sum, byte| sum.wrapping_add(byte as u16));

                Ok(ReadResult::Segments(Self::ConfigSlots {
                    data: Data {
                        registration,
                        primary_dns,
                        secondary_dns,
                        login_id,
                        email,
                        smtp_server,
                        pop_server,
                    },
                    checksum,
                }))
            }
            Self::ConfigSlots { data, mut checksum } => {
                // Configuration slots.
                let configuration_slots = [
                    Self::read_configuration_slot(bytes, RangedUsize::new_static::<0>()),
                    Self::read_configuration_slot(bytes, RangedUsize::new_static::<1>()),
                    Self::read_configuration_slot(bytes, RangedUsize::new_static::<2>()),
                ];

                // Checksum.
                checksum = checksum.wrapping_add(
                    bytes[..0x71]
                        .iter()
                        .copied()
                        .fold(0u16, |sum, byte| sum.wrapping_add(byte as u16)),
                );
                let received_checksum = ((bytes[77] as u16) << 8) | (bytes[78] as u16);
                if checksum != received_checksum {
                    return Err(Error::Checksum {
                        calculated: checksum,
                        received: received_checksum,
                    });
                }

                Ok(ReadResult::Success(Config {
                    registration: data.registration,
                    primary_dns: data.primary_dns,
                    secondary_dns: data.secondary_dns,
                    login_id: data.login_id,
                    email: data.email,
                    smtp_server: data.smtp_server,
                    pop_server: data.pop_server,
                    configuration_slots,
                }))
            }
        }
    }
}
