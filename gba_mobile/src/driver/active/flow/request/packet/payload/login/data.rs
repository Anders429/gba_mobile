use core::net::Ipv4Addr;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct Response {
    pub(super) ip: Ipv4Addr,
    pub(super) primary_dns: Ipv4Addr,
    pub(super) secondary_dns: Ipv4Addr,
}

#[derive(Debug)]
enum Addr {
    Octet1,
    Octet2(u8),
    Octet3(u8, u8),
    Octet4(u8, u8, u8),
}

impl Addr {
    fn new() -> Self {
        Self::Octet1
    }

    fn receive_data(self, byte: u8) -> Either<Self, Ipv4Addr> {
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

#[derive(Debug)]
pub(in crate::driver) enum Data {
    Ip(Addr),
    PrimaryDns {
        ip: Ipv4Addr,
        addr: Addr,
    },
    SecondaryDns {
        ip: Ipv4Addr,
        primary_dns: Ipv4Addr,
        addr: Addr,
    },
}

impl Data {
    pub(super) fn new() -> Self {
        Self::Ip(Addr::new())
    }

    pub(super) fn receive_data(self, byte: u8) -> Either<Self, Response> {
        match self {
            Self::Ip(addr) => match addr.receive_data(byte) {
                Either::Left(addr) => Either::Left(Self::Ip(addr)),
                Either::Right(ip) => Either::Left(Self::PrimaryDns {
                    ip,
                    addr: Addr::new(),
                }),
            },
            Self::PrimaryDns { ip, addr } => match addr.receive_data(byte) {
                Either::Left(addr) => Either::Left(Self::PrimaryDns { ip, addr }),
                Either::Right(primary_dns) => Either::Left(Self::SecondaryDns {
                    ip,
                    primary_dns,
                    addr: Addr::new(),
                }),
            },
            Self::SecondaryDns {
                ip,
                primary_dns,
                addr,
            } => match addr.receive_data(byte) {
                Either::Left(addr) => Either::Left(Self::SecondaryDns {
                    ip,
                    primary_dns,
                    addr,
                }),
                Either::Right(secondary_dns) => Either::Right(Response {
                    ip,
                    primary_dns,
                    secondary_dns,
                }),
            },
        }
    }
}
