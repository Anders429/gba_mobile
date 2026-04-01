mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::request::{Packet, packet::payload};
use crate::{Socket, Timer, driver::Adapter, mmio::serial::TransferLength, socket};
use either::Either;
use embedded_io::{Error as _, Write};

#[derive(Debug)]
pub(in super::super) struct TransferData {
    packet: Packet<payload::TransferData>,
}

impl TransferData {
    pub(super) fn new<Buffer>(
        transfer_length: TransferLength,
        timer: Timer,
        socket: &mut Socket<Buffer>,
    ) -> Self {
        Self {
            packet: Packet::new(
                payload::TransferData::new(socket.id, socket.write_buffer.take()),
                transfer_length,
                timer,
            ),
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        self.packet
            .vblank()
            .map(|packet| Self { packet })
            .map_err(Timeout::TransferData)
    }

    pub(super) fn timer(&mut self) {
        self.packet.timer();
    }

    pub(super) fn serial<Buffer>(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        socket: &mut Socket<Buffer>,
    ) -> Result<Option<Self>, Error>
    where
        Buffer: Write,
    {
        match self.packet.serial(timer).map_err(Error::TransferData)? {
            Either::Left(packet) => Ok(Some(Self { packet })),
            Either::Right(response) => {
                *adapter = response.adapter;
                if matches!(socket.status, socket::Status::Connected) {
                    match response.payload.response {
                        payload::transfer_data::Response::Data(data) => {
                            socket.frame = 0;
                            socket
                                .read_buffer
                                .write_all(data.as_slice())
                                .map_err(|error| Error::Buffer(error.kind()))?;
                        }
                        payload::transfer_data::Response::FinalData(data) => {
                            socket.frame = 0;
                            socket.status = socket::Status::ClosedRemotely;
                            socket
                                .read_buffer
                                .write_all(data.as_slice())
                                .map_err(|error| Error::Buffer(error.kind()))?;
                        }
                        payload::transfer_data::Response::ConnectionFailed => {
                            socket.status = socket::Status::ConnectionLost;
                        }
                    }
                }
                Ok(None)
            }
        }
    }
}
