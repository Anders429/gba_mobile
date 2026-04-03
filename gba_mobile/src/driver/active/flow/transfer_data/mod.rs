mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::request::{Packet, packet::payload};
use crate::{
    ArrayVec, Socket, Timer,
    driver::{Adapter, active::flow::request::RepeatingIdle},
    mmio::serial::TransferLength,
    socket,
};
use either::Either;

#[derive(Debug)]
pub(in super::super) enum TransferData {
    TransferData(Packet<payload::TransferData>),
    WriteToBuffer(ArrayVec<u8, 254>, u8, RepeatingIdle),
}

impl TransferData {
    pub(super) fn new<Buffer>(
        transfer_length: TransferLength,
        timer: Timer,
        socket: &mut Socket<Buffer>,
    ) -> Self {
        Self::TransferData(Packet::new(
            payload::TransferData::new(socket.id, socket.write_buffer.take()),
            transfer_length,
            timer,
        ))
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::TransferData(packet) => packet
                .vblank()
                .map(|packet| Self::TransferData(packet))
                .map_err(Timeout::TransferData),
            Self::WriteToBuffer(buffer, index, repeating_idle) => repeating_idle
                .vblank()
                .map(|repeating_idle| Self::WriteToBuffer(buffer, index, repeating_idle))
                .map_err(Timeout::WriteToBuffer),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::TransferData(packet) => packet.timer(),
            Self::WriteToBuffer(_, _, repeating_idle) => repeating_idle.timer(),
        }
    }

    pub(super) fn serial<Buffer>(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: TransferLength,
        socket: &mut Socket<Buffer>,
    ) -> Result<Option<Self>, Error>
    where
        Buffer: socket::Buffer,
    {
        match self {
            Self::TransferData(packet) => {
                match packet.serial(timer).map_err(Error::TransferData)? {
                    Either::Left(packet) => Ok(Some(Self::TransferData(packet))),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        if matches!(socket.status, socket::Status::Connected) {
                            match response.payload.response {
                                payload::transfer_data::Response::Data(data) => {
                                    socket.frame = 0;
                                    Ok(Some(Self::WriteToBuffer(
                                        data,
                                        0,
                                        RepeatingIdle::new(transfer_length, timer),
                                    )))
                                }
                                payload::transfer_data::Response::FinalData(data) => {
                                    socket.frame = 0;
                                    socket.status = socket::Status::ClosedRemotely;
                                    Ok(Some(Self::WriteToBuffer(
                                        data,
                                        0,
                                        RepeatingIdle::new(transfer_length, timer),
                                    )))
                                }
                                payload::transfer_data::Response::ConnectionFailed => {
                                    socket.status = socket::Status::ConnectionLost;
                                    Ok(None)
                                }
                            }
                        } else {
                            Ok(None)
                        }
                    }
                }
            }
            Self::WriteToBuffer(buffer, index, repeating_idle) => {
                let repeating_idle = repeating_idle.serial(timer).map_err(Error::Idle)?;

                let bytes_written = socket
                    .read_buffer
                    .write(buffer.as_slice().get((index as usize)..).unwrap_or(&[]))
                    .map_err(|_| Error::WriteToBuffer)?;
                let index = index.saturating_add(bytes_written as u8);

                if buffer.len() <= index {
                    Ok(None)
                } else {
                    Ok(Some(Self::WriteToBuffer(buffer, index, repeating_idle)))
                }
            }
        }
    }
}
