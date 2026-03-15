pub(in crate::driver::active::flow) mod accept_connection;
pub(in crate::driver::active::flow) mod begin_session;
pub(in crate::driver::active::flow) mod connect;
pub(in crate::driver::active::flow) mod enable_sio32;
pub(in crate::driver::active::flow) mod end_session;
pub(in crate::driver::active::flow) mod reset;

mod command_error;
mod error;

pub(in crate::driver) use error::Error;

pub(in crate::driver::active::flow) use accept_connection::AcceptConnection;
pub(in crate::driver::active::flow) use begin_session::BeginSession;
pub(in crate::driver::active::flow) use connect::Connect;
pub(in crate::driver::active::flow) use enable_sio32::EnableSio32;
pub(in crate::driver::active::flow) use end_session::EndSession;
pub(in crate::driver::active::flow) use reset::Reset;

use crate::driver::Command;
use core::{fmt::Debug, num::NonZeroU16};
use either::Either;

pub(in crate::driver::active::flow) trait Payload:
    Debug + 'static
{
    type Send: Send<ReceiveCommand = Self::ReceiveCommand>;

    type ReceiveCommand: ReceiveCommand<ReceiveLength = Self::ReceiveLength>;
    type ReceiveLength: ReceiveLength<
            ReceiveCommand = Self::ReceiveCommand,
            ReceiveData = Self::ReceiveData,
            ReceiveParsed = Self::ReceiveParsed,
        >;
    type ReceiveData: ReceiveData<ReceiveCommand = Self::ReceiveCommand, ReceiveParsed = Self::ReceiveParsed>;
    type ReceiveParsed: ReceiveParsed<ReceiveCommand = Self::ReceiveCommand>;
}

pub(in crate::driver::active::flow) trait Send: Debug {
    type ReceiveCommand;

    fn command(&self) -> Command;
    fn length(&self) -> u8;
    fn get(&self, index: u8) -> u8;

    fn finish(self) -> Self::ReceiveCommand;
}

pub(in crate::driver::active::flow) trait ReceiveCommand:
    Sized + Debug
{
    type ReceiveLength;
    type Error: core::error::Error + 'static + Clone;

    fn receive_command(self, command: Command) -> Result<Self::ReceiveLength, (Self::Error, Self)>;
}

pub(in crate::driver::active::flow) trait ReceiveLength:
    Debug
{
    type ReceiveCommand;
    type ReceiveData;
    type ReceiveParsed;
    type Error: core::error::Error + 'static + Clone;

    fn receive_length(
        self,
        length: u8,
    ) -> Result<Either<Self::ReceiveData, Self::ReceiveParsed>, (Self::Error, Self::ReceiveCommand)>;

    fn restart(self) -> Self::ReceiveCommand;
}

pub(in crate::driver::active::flow) trait ReceiveData:
    Sized + Debug
{
    type ReceiveCommand;
    type ReceiveParsed;
    type Error: core::error::Error + 'static + Clone;

    fn receive_data(
        self,
        data: u8,
    ) -> Result<
        Either<Self, Self::ReceiveParsed>,
        (Self::Error, Self::ReceiveCommand, Option<(NonZeroU16, u16)>),
    >;
}

pub(in crate::driver::active::flow) trait ReceiveParsed:
    Debug
{
    type ReceiveCommand;

    fn command(&self) -> Command;

    fn restart(self) -> Self::ReceiveCommand;
}
