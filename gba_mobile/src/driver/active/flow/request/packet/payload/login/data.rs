use super::super::addr;
use core::net::Ipv4Addr;
use either::Either;

#[derive(Debug)]
pub(in crate::driver) struct Response {
    pub(super) ip: Ipv4Addr,
    pub(super) primary_dns: Ipv4Addr,
    pub(super) secondary_dns: Ipv4Addr,
}

#[derive(Debug)]
pub(in crate::driver) enum Data {
    Ip(addr::Data),
    PrimaryDns {
        ip: Ipv4Addr,
        addr: addr::Data,
    },
    SecondaryDns {
        ip: Ipv4Addr,
        primary_dns: Ipv4Addr,
        addr: addr::Data,
    },
}

impl Data {
    pub(super) fn new() -> Self {
        Self::Ip(addr::Data::new())
    }

    pub(super) fn receive_data(self, byte: u8) -> Either<Self, Response> {
        match self {
            Self::Ip(addr) => match addr.receive_data(byte) {
                Either::Left(addr) => Either::Left(Self::Ip(addr)),
                Either::Right(ip) => Either::Left(Self::PrimaryDns {
                    ip,
                    addr: addr::Data::new(),
                }),
            },
            Self::PrimaryDns { ip, addr } => match addr.receive_data(byte) {
                Either::Left(addr) => Either::Left(Self::PrimaryDns { ip, addr }),
                Either::Right(primary_dns) => Either::Left(Self::SecondaryDns {
                    ip,
                    primary_dns,
                    addr: addr::Data::new(),
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
