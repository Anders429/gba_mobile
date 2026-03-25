use core::net::Ipv4Addr;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) enum Data {
    Octet1,
    Octet2(u8),
    Octet3(u8, u8),
    Octet4(u8, u8, u8),
}

impl Data {
    pub(super) fn new() -> Self {
        Self::Octet1
    }

    pub(super) fn receive_data(self, byte: u8) -> Either<Self, Ipv4Addr> {
        match self {
            Self::Octet1 => Either::Left(Self::Octet2(byte)),
            Self::Octet2(octet1) => Either::Left(Self::Octet3(octet1, byte)),
            Self::Octet3(octet1, octet2) => Either::Left(Self::Octet4(octet1, octet2, byte)),
            Self::Octet4(octet1, octet2, octet3) => {
                Either::Right(Ipv4Addr::from_octets([octet1, octet2, octet3, byte]))
            }
        }
    }
}
