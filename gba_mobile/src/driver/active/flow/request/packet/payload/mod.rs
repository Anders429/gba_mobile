pub(in crate::driver::active::flow) mod accept_connection;
pub(in crate::driver::active::flow) mod begin_session;
pub(in crate::driver::active::flow) mod close_tcp;
pub(in crate::driver::active::flow) mod close_udp;
pub(in crate::driver::active::flow) mod connect;
pub(in crate::driver::active::flow) mod connection_status;
pub(in crate::driver::active::flow) mod disconnect;
pub(in crate::driver::active::flow) mod dns;
pub(in crate::driver::active::flow) mod enable_sio32;
pub(in crate::driver::active::flow) mod end_session;
pub(in crate::driver::active::flow) mod login;
pub(in crate::driver::active::flow) mod open_tcp;
pub(in crate::driver::active::flow) mod open_udp;
pub(in crate::driver::active::flow) mod read_config;
pub(in crate::driver::active::flow) mod reset;
pub(in crate::driver::active::flow) mod transfer_data;
pub(in crate::driver::active::flow) mod write_config;

mod addr;
mod command_error;
mod error;

pub(in crate::driver) use error::Error;

pub(in crate::driver::active::flow) use accept_connection::AcceptConnection;
pub(in crate::driver::active::flow) use begin_session::BeginSession;
pub(in crate::driver::active::flow) use close_tcp::CloseTcp;
pub(in crate::driver::active::flow) use close_udp::CloseUdp;
pub(in crate::driver::active::flow) use connect::Connect;
pub(in crate::driver::active::flow) use connection_status::ConnectionStatus;
pub(in crate::driver::active::flow) use disconnect::Disconnect;
pub(in crate::driver::active::flow) use dns::Dns;
pub(in crate::driver::active::flow) use enable_sio32::EnableSio32;
pub(in crate::driver::active::flow) use end_session::EndSession;
pub(in crate::driver::active::flow) use login::Login;
pub(in crate::driver::active::flow) use open_tcp::OpenTcp;
pub(in crate::driver::active::flow) use open_udp::OpenUdp;
pub(in crate::driver::active::flow) use read_config::ReadConfig;
pub(in crate::driver::active::flow) use reset::Reset;
pub(in crate::driver::active::flow) use transfer_data::TransferData;
pub(in crate::driver::active::flow) use write_config::WriteConfig;

use super::Data;
use core::fmt::Debug;

pub(in crate::driver) trait Payload: Debug {
    type Response<'a>;
    type Error: core::error::Error + Clone + 'static;

    fn parse<'a>(self, data: &'a Data) -> Result<Self::Response<'a>, Self::Error>;
}
