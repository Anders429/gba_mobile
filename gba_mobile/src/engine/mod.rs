mod adapter;
mod command;
mod error;
mod flow;
mod packet;
mod sink;
mod source;

use core::num::{NonZeroU8, NonZeroU16};

pub(crate) use error::Error;

use crate::mmio::serial::{SIODATA8, SIODATA32, TransferLength};
use adapter::Adapter;
use command::Command;
use either::Either;
use packet::{Packet, receive, send};
use source::Source;

/// Handshake for beginning a session.
const HANDSHAKE: [u8; 8] = [0x4e, 0x49, 0x4e, 0x54, 0x45, 0x4e, 0x44, 0x4f];

#[derive(Debug)]
enum State {
    NotConnected,
    LinkingP2P {
        adapter: Adapter,
        transfer_length: TransferLength,

        packet: Option<Packet>,
        flow: flow::LinkingP2P,
    },
    P2P,
    Error(Error),
}

#[derive(Debug)]
pub(crate) struct Engine {
    state: State,
}

impl Engine {
    /// Create a new packet engine.
    pub(crate) const fn new() -> Self {
        Self {
            state: State::NotConnected,
        }
    }

    pub(crate) fn link_p2p(&mut self) {
        // TODO: Close any previous sessions.
        self.state = State::LinkingP2P {
            adapter: Adapter::Blue,
            transfer_length: TransferLength::_8Bit,

            packet: None,
            flow: flow::LinkingP2P::Waking,
        }
    }

    pub(crate) fn vblank(&mut self) {
        match &mut self.state {
            State::NotConnected => {}
            State::LinkingP2P {
                transfer_length,
                packet,
                flow,
                ..
            } => {
                if let Some(_) = packet {
                    // TODO: Handle requests.
                } else {
                    // Schedule a new request.
                    *packet = Some(flow.request(*transfer_length));
                }
            }
            State::P2P => todo!(),
            State::Error(_) => {}
        }
    }

    pub(crate) fn timer(&mut self) {
        if let State::LinkingP2P { packet, .. } = &mut self.state
            && let Some(packet) = packet.as_mut()
        {
            match packet {
                // /-----------\
                // | SIO8 Send |
                // \-----------/
                Packet::Send8 {
                    step,
                    source,
                    checksum,
                    ..
                } => {
                    let byte = match step {
                        send::Step8::MagicByte1 => 0x99,
                        send::Step8::MagicByte2 => 0x66,
                        send::Step8::HeaderCommand => {
                            let byte = source.command() as u8;
                            *checksum = checksum.wrapping_add(byte as u16);
                            byte
                        }
                        send::Step8::HeaderEmptyByte => 0x00,
                        send::Step8::HeaderLength1 => 0x00,
                        send::Step8::HeaderLength2 => {
                            let byte = source.length();
                            *checksum = checksum.wrapping_add(byte as u16);
                            byte
                        }
                        send::Step8::Data { index } => {
                            let byte = source.get(*index);
                            *checksum = checksum.wrapping_add(byte as u16);
                            byte
                        }
                        send::Step8::Checksum1 => (*checksum >> 8) as u8,
                        send::Step8::Checksum2 => *checksum as u8,
                        send::Step8::AcknowledgementSignalDevice => 0x81,
                        send::Step8::AcknowledgementSignalCommand => 0x00,
                    };

                    unsafe { SIODATA8.write_volatile(byte) };
                }

                // /------------\
                // | SIO32 Send |
                // \------------/
                Packet::Send32 {
                    step,
                    source,
                    checksum,
                    ..
                } => {
                    let bytes = match step {
                        send::Step32::MagicByte => {
                            let command = source.command() as u8;
                            *checksum = checksum.wrapping_add(command as u16);
                            u32::from_be_bytes([0x99, 0x66, command, 0x00])
                        }
                        send::Step32::HeaderLength => {
                            let length = source.length();
                            *checksum = checksum.wrapping_add(length as u16);
                            if length == 0 {
                                // If not sending any data, we skip straight to sending the checksum.
                                u32::from_be_bytes([
                                    0x00,
                                    length,
                                    (*checksum >> 8) as u8,
                                    *checksum as u8,
                                ])
                            } else {
                                let data_0 = source.get(0);
                                let data_1 = source.get(1);
                                *checksum = checksum
                                    .wrapping_add(data_0 as u16)
                                    .wrapping_add(data_1 as u16);
                                u32::from_be_bytes([0x00, length, data_0, data_1])
                            }
                        }
                        send::Step32::Data { index } => {
                            let length = source.length();
                            let mut bytes = [0x00; 4];
                            let mut offset = 0;
                            while offset < 4 && *index + offset < length {
                                let byte = source.get(*index + offset);
                                *checksum = checksum.wrapping_add(byte as u16);
                                bytes[offset as usize] = byte;
                                offset += 1;
                            }
                            if offset < 3 {
                                // If we have room, we pack the checksum in as well.
                                bytes[2] = (*checksum >> 8) as u8;
                                bytes[3] = *checksum as u8;
                            }
                            u32::from_be_bytes(bytes)
                        }
                        send::Step32::Checksum => u32::from_be_bytes([
                            0x00,
                            0x00,
                            (*checksum >> 8) as u8,
                            *checksum as u8,
                        ]),
                        send::Step32::AcknowledgementSignal => 0x81_00_00_00,
                    };

                    unsafe { SIODATA32.write_volatile(bytes) };
                }

                // /--------------\
                // | SIO8 Receive |
                // \--------------/
                Packet::Receive8 { step, .. } => {
                    let byte = match step {
                        receive::Step8::AcknowledgementSignalDevice { .. } => 0x81,
                        receive::Step8::AcknowledgementSignalCommand { result, .. } => {
                            result.command() as u8 ^ 0x80
                        }
                        _ => 0x4b,
                    };
                    unsafe { SIODATA8.write_volatile(byte) };
                }

                // /---------------\
                // | SIO32 Receive |
                // \---------------/
                Packet::Receive32 { step, .. } => {
                    let bytes = match step {
                        receive::Step32::AcknowledgementSignal { result, .. } => {
                            u32::from_be_bytes([0x81, (result.command() as u8 ^ 0x80), 0x00, 0x00])
                        }
                        _ => 0x4b_4b_4b_4b,
                    };
                    unsafe { SIODATA32.write_volatile(bytes) };
                }

                // /--------------------\
                // | SIO8 Receive Error |
                // \--------------------/
                Packet::Receive8Error {
                    step,
                    error,
                    attempt,
                    ..
                } => {
                    let byte = match step {
                        receive::Step8Error::AcknowledgementSignalDevice { .. } => 0x81,
                        receive::Step8Error::AcknowledgementSignalCommand { .. } => {
                            if *attempt + 1 < packet::MAX_RETRIES {
                                error.command() as u8 ^ 0x80
                            } else {
                                // Since we've errored on communication too much, it doesn't matter
                                // what we send here. We are going to error out the link session
                                // anyway.
                                Command::Empty as u8 ^ 0x80
                            }
                        }
                        _ => 0x4b,
                    };
                    unsafe { SIODATA8.write_volatile(byte) };
                }

                // /---------------------\
                // | SIO32 Receive Error |
                // \---------------------/
                Packet::Receive32Error {
                    step,
                    error,
                    attempt,
                    ..
                } => {
                    let bytes = match step {
                        receive::Step32Error::AcknowledgementSignal { .. } => {
                            let command_byte = if *attempt + 1 < packet::MAX_RETRIES {
                                error.command() as u8 ^ 0x80
                            } else {
                                // Since we've errored on communication too much, it doesn't matter
                                // what we send here. We are going to error out the link session
                                // anyway.
                                Command::Empty as u8 ^ 0x80
                            };
                            u32::from_be_bytes([0x81, command_byte, 0x00, 0x00])
                        }
                        _ => 0x4b_4b_4b_4b,
                    };
                    unsafe { SIODATA32.write_volatile(bytes) };
                }
            }
        }
    }

    pub(crate) fn serial(&mut self) {
        if let State::LinkingP2P {
            adapter,
            transfer_length,
            packet: state_packet,
            flow,
        } = &mut self.state
            && let Some(packet) = state_packet.take()
        {
            *state_packet = match packet {
                // /-----------\
                // | SIO8 Send |
                // \-----------/
                Packet::Send8 {
                    step,
                    source,
                    checksum,
                    attempt,
                } => {
                    let byte = unsafe { SIODATA8.read_volatile() };
                    match step {
                        send::Step8::MagicByte1 => Some(Packet::Send8 {
                            step: send::Step8::MagicByte2,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step8::MagicByte2 => Some(Packet::Send8 {
                            step: send::Step8::HeaderCommand,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step8::HeaderCommand => Some(Packet::Send8 {
                            step: send::Step8::HeaderEmptyByte,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step8::HeaderEmptyByte => Some(Packet::Send8 {
                            step: send::Step8::HeaderLength1,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step8::HeaderLength1 => Some(Packet::Send8 {
                            step: send::Step8::HeaderLength2,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step8::HeaderLength2 => {
                            if source.length() > 0 {
                                Some(Packet::Send8 {
                                    step: send::Step8::Data { index: 0 },
                                    source,
                                    checksum,
                                    attempt,
                                })
                            } else {
                                Some(Packet::Send8 {
                                    step: send::Step8::Checksum1,
                                    source,
                                    checksum,
                                    attempt,
                                })
                            }
                        }
                        send::Step8::Data { index } => {
                            let next_index = index + 1;
                            if source.length() > next_index {
                                Some(Packet::Send8 {
                                    step: send::Step8::Data { index: next_index },
                                    source,
                                    checksum,
                                    attempt,
                                })
                            } else {
                                Some(Packet::Send8 {
                                    step: send::Step8::Checksum1,
                                    source,
                                    checksum,
                                    attempt,
                                })
                            }
                        }
                        send::Step8::Checksum1 => Some(Packet::Send8 {
                            step: send::Step8::Checksum2,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step8::Checksum2 { .. } => Some(Packet::Send8 {
                            step: send::Step8::AcknowledgementSignalDevice,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step8::AcknowledgementSignalDevice => Some(Packet::Send8 {
                            step: send::Step8::AcknowledgementSignalCommand,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step8::AcknowledgementSignalCommand => {
                            let new_attempt = attempt + 1;
                            match Command::try_from(byte ^ 0x80) {
                                Ok(
                                    Command::NotSupportedError
                                    | Command::MalformedError
                                    | Command::InternalError,
                                ) if new_attempt < packet::MAX_RETRIES => {
                                    // Retry.
                                    Some(Packet::Send8 {
                                        step: send::Step8::MagicByte1,
                                        source,
                                        checksum: 0,
                                        attempt: new_attempt,
                                    })
                                }
                                Ok(Command::NotSupportedError) => {
                                    // Too many retries. Stop trying and set error state.
                                    self.state = State::Error(Error::Packet(packet::Error::Send(
                                        packet::send::Error::UnsupportedCommand(source.command()),
                                    )));
                                    return;
                                }
                                Ok(Command::MalformedError) => {
                                    // Too many retries. Stop trying and set error state.
                                    self.state = State::Error(Error::Packet(packet::Error::Send(
                                        packet::send::Error::Malformed,
                                    )));
                                    return;
                                }
                                Ok(Command::InternalError) => {
                                    // Too many retries. Stop trying and set error state.
                                    self.state = State::Error(Error::Packet(packet::Error::Send(
                                        packet::send::Error::AdapterInternalError,
                                    )));
                                    return;
                                }
                                _ => {
                                    // We don't verify anything here and simply assume the adapter
                                    // responded with a correct command. If the adapter is in an invalid
                                    // state, we will find out when receiving the response packet instead.
                                    Some(Packet::Receive8 {
                                        step: receive::Step8::MagicByte1 {
                                            sink: source.sink(),
                                        },
                                        checksum: 0,

                                        attempt: 0,
                                    })
                                }
                            }
                        }
                    }
                }

                // /------------\
                // | SIO32 Send |
                // \------------/
                Packet::Send32 {
                    step,
                    source,
                    checksum,
                    attempt,
                } => {
                    let bytes = unsafe { SIODATA32.read_volatile() };
                    match step {
                        send::Step32::MagicByte => Some(Packet::Send32 {
                            step: send::Step32::HeaderLength,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step32::HeaderLength => {
                            let new_step = match source.length() {
                                0 => send::Step32::AcknowledgementSignal,
                                1..=2 => send::Step32::Checksum,
                                _ => send::Step32::Data { index: 2 },
                            };
                            Some(Packet::Send32 {
                                step: new_step,
                                source,
                                checksum,
                                attempt,
                            })
                        }
                        send::Step32::Data { index } => {
                            let length = source.length();
                            let new_step = if index + 2 >= length {
                                // We can fit the checksum in here.
                                send::Step32::AcknowledgementSignal
                            } else if index + 4 >= length {
                                send::Step32::Checksum
                            } else {
                                send::Step32::Data { index: index + 4 }
                            };
                            Some(Packet::Send32 {
                                step: new_step,
                                source,
                                checksum,
                                attempt,
                            })
                        }
                        send::Step32::Checksum => Some(Packet::Send32 {
                            step: send::Step32::AcknowledgementSignal,
                            source,
                            checksum,
                            attempt,
                        }),
                        send::Step32::AcknowledgementSignal => {
                            let new_attempt = attempt + 1;
                            match Command::try_from(bytes.to_be_bytes()[1] ^ 0x80) {
                                Ok(
                                    Command::NotSupportedError
                                    | Command::MalformedError
                                    | Command::InternalError,
                                ) if new_attempt < packet::MAX_RETRIES => {
                                    // Retry.
                                    Some(Packet::Send32 {
                                        step: send::Step32::MagicByte,
                                        source,
                                        checksum: 0,
                                        attempt: new_attempt,
                                    })
                                }
                                Ok(Command::NotSupportedError) => {
                                    // Too many retries. Stop trying and set error state.
                                    self.state = State::Error(Error::Packet(packet::Error::Send(
                                        packet::send::Error::UnsupportedCommand(source.command()),
                                    )));
                                    return;
                                }
                                Ok(Command::MalformedError) => {
                                    // Too many retries. Stop trying and set error state.
                                    self.state = State::Error(Error::Packet(packet::Error::Send(
                                        packet::send::Error::Malformed,
                                    )));
                                    return;
                                }
                                Ok(Command::InternalError) => {
                                    // Too many retries. Stop trying and set error state.
                                    self.state = State::Error(Error::Packet(packet::Error::Send(
                                        packet::send::Error::AdapterInternalError,
                                    )));
                                    return;
                                }
                                _ => {
                                    // We don't verify anything here and simply assume the adapter
                                    // responded with a correct command. If the adapter is in an invalid
                                    // state, we will find out when receiving the response packet instead.
                                    Some(Packet::Receive32 {
                                        step: receive::Step32::MagicByte {
                                            sink: source.sink(),
                                        },
                                        checksum: 0,

                                        attempt: 0,
                                    })
                                }
                            }
                        }
                    }
                }

                // /--------------\
                // | SIO8 Receive |
                // \--------------/
                Packet::Receive8 {
                    step,
                    checksum,
                    attempt,
                } => {
                    let byte = unsafe { SIODATA8.read_volatile() };
                    match step {
                        receive::Step8::MagicByte1 { sink } => match byte {
                            0x99 => Some(Packet::Receive8 {
                                step: receive::Step8::MagicByte2 { sink },
                                checksum,
                                attempt,
                            }),
                            0xd2 => Some(Packet::Receive8 {
                                step: receive::Step8::MagicByte1 { sink },
                                checksum,
                                attempt,
                            }),
                            _ => Some(Packet::Receive8Error {
                                step: receive::Step8Error::MagicByte2 { sink },
                                error: packet::receive::Error::MagicValue1(byte),
                                attempt,
                            }),
                        },
                        receive::Step8::MagicByte2 { sink } => match byte {
                            0x66 => Some(Packet::Receive8 {
                                step: receive::Step8::HeaderCommand { sink },
                                checksum,
                                attempt,
                            }),
                            _ => Some(Packet::Receive8Error {
                                step: receive::Step8Error::HeaderCommand { sink },
                                error: packet::receive::Error::MagicValue2(byte),
                                attempt,
                            }),
                        },
                        receive::Step8::HeaderCommand { sink } => match Command::try_from(byte) {
                            Ok(command) => match sink.parse(command) {
                                Ok(sink) => Some(Packet::Receive8 {
                                    step: receive::Step8::HeaderEmptyByte { sink },
                                    checksum: checksum.wrapping_add(byte as u16),
                                    attempt,
                                }),
                                Err((error, sink)) => Some(Packet::Receive8Error {
                                    step: receive::Step8Error::HeaderEmptyByte { sink },
                                    error: packet::receive::Error::UnsupportedCommand(error),
                                    attempt,
                                }),
                            },
                            Err(unknown) => Some(Packet::Receive8Error {
                                step: receive::Step8Error::HeaderEmptyByte { sink },
                                error: packet::receive::Error::UnknownCommand(unknown),
                                attempt,
                            }),
                        },
                        receive::Step8::HeaderEmptyByte { sink } => Some(Packet::Receive8 {
                            step: receive::Step8::HeaderLength1 { sink },
                            checksum: checksum.wrapping_add(byte as u16),
                            attempt,
                        }),
                        receive::Step8::HeaderLength1 { sink } => Some(Packet::Receive8 {
                            step: receive::Step8::HeaderLength2 {
                                first_byte: byte,
                                sink,
                            },
                            checksum: checksum.wrapping_add(byte as u16),
                            attempt,
                        }),
                        receive::Step8::HeaderLength2 { sink, first_byte } => {
                            let full_length = ((first_byte as u16) << 8) | (byte as u16);
                            match sink.parse(full_length) {
                                Ok(Either::Left(sink)) => Some(Packet::Receive8 {
                                    step: receive::Step8::Data { sink },
                                    checksum: checksum.wrapping_add(byte as u16),
                                    attempt,
                                }),
                                Ok(Either::Right(result)) => Some(Packet::Receive8 {
                                    step: receive::Step8::Checksum1 { result },
                                    checksum: checksum.wrapping_add(byte as u16),
                                    attempt,
                                }),
                                Err((error, sink)) => match NonZeroU16::new(full_length) {
                                    Some(length) => Some(Packet::Receive8Error {
                                        step: receive::Step8Error::Data {
                                            sink,
                                            index: 0,
                                            length,
                                        },
                                        error: packet::receive::Error::UnexpectedLength(error),
                                        attempt,
                                    }),
                                    None => Some(Packet::Receive8Error {
                                        step: receive::Step8Error::Checksum1 { sink },
                                        error: packet::receive::Error::UnexpectedLength(error),
                                        attempt,
                                    }),
                                },
                            }
                        }
                        receive::Step8::Data { sink } => match sink.parse(byte) {
                            Ok(Either::Left(sink)) => Some(Packet::Receive8 {
                                step: receive::Step8::Data { sink },
                                checksum: checksum.wrapping_add(byte as u16),
                                attempt,
                            }),
                            Ok(Either::Right(result)) => Some(Packet::Receive8 {
                                step: receive::Step8::Checksum1 { result },
                                checksum: checksum.wrapping_add(byte as u16),
                                attempt,
                            }),
                            Err((error, index, length, sink)) => {
                                if let Some(next_index) = index.checked_add(1)
                                    && next_index < length.get()
                                {
                                    // We still have more data to receive in the error state.
                                    Some(Packet::Receive8Error {
                                        step: receive::Step8Error::Data {
                                            sink,
                                            index: next_index,
                                            length,
                                        },
                                        error: packet::receive::Error::MalformedData(error),
                                        attempt,
                                    })
                                } else {
                                    // The error happened on the last byte being received.
                                    Some(Packet::Receive8Error {
                                        step: receive::Step8Error::Checksum1 { sink },
                                        error: packet::receive::Error::MalformedData(error),
                                        attempt,
                                    })
                                }
                            }
                        },
                        receive::Step8::Checksum1 { result } => Some(Packet::Receive8 {
                            step: receive::Step8::Checksum2 {
                                first_byte: byte,
                                result,
                            },
                            checksum,
                            attempt,
                        }),
                        receive::Step8::Checksum2 { result, first_byte } => {
                            let full_checksum = ((first_byte as u16) << 8) | (byte as u16);
                            if full_checksum == checksum {
                                Some(Packet::Receive8 {
                                    step: receive::Step8::AcknowledgementSignalDevice { result },
                                    checksum,
                                    attempt,
                                })
                            } else {
                                Some(Packet::Receive8Error {
                                    step: receive::Step8Error::AcknowledgementSignalDevice {
                                        sink: result.revert(),
                                    },
                                    error: packet::receive::Error::Checksum {
                                        calculated: checksum,
                                        received: full_checksum,
                                    },
                                    attempt,
                                })
                            }
                        }
                        receive::Step8::AcknowledgementSignalDevice { result } => {
                            match Adapter::try_from(byte) {
                                Ok(received_adapter) => Some(Packet::Receive8 {
                                    step: receive::Step8::AcknowledgementSignalCommand {
                                        result,
                                        adapter: received_adapter,
                                    },
                                    checksum,
                                    attempt,
                                }),
                                Err(unknown) => Some(Packet::Receive8Error {
                                    step: receive::Step8Error::AcknowledgementSignalCommand {
                                        sink: result.revert(),
                                    },
                                    error: packet::receive::Error::UnsupportedDevice(unknown),
                                    attempt,
                                }),
                            }
                        }
                        receive::Step8::AcknowledgementSignalCommand {
                            result,
                            adapter: received_adapter,
                        } => {
                            // The acknowledgement signal command we receive is expected to be 0x00.
                            match NonZeroU8::new(byte) {
                                None => {
                                    // We don't care about what the adapter was set to previously.
                                    // We just want to store whatever type it's currently telling
                                    // us it is.
                                    *adapter = received_adapter;
                                    match result.finish() {
                                        sink::Finished::Success => {
                                            if let Some(new_flow) = flow.next() {
                                                *flow = new_flow;
                                            } else {
                                                self.state = State::P2P;
                                                return;
                                            }
                                        }
                                    }
                                    None
                                }
                                Some(nonzero) => {
                                    // We can no longer retry at this point. We simply enter an
                                    // error state.
                                    self.state =
                                        State::Error(Error::Packet(packet::Error::Receive(
                                            packet::receive::Error::NonZeroAcknowledgementCommand(
                                                nonzero,
                                            ),
                                        )));
                                    return;
                                }
                            }
                        }
                    }
                }

                // /---------------\
                // | SIO32 Receive |
                // \---------------/
                Packet::Receive32 {
                    step,
                    checksum,
                    attempt,
                } => {
                    let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };
                    match step {
                        receive::Step32::MagicByte { sink } => match bytes[0] {
                            0xd2 => Some(Packet::Receive32 {
                                step: receive::Step32::MagicByte { sink },
                                checksum,
                                attempt,
                            }),
                            0x99 => match bytes[1] {
                                0x66 => match Command::try_from(bytes[2]) {
                                    Ok(command) => match sink.parse(command) {
                                        Ok(sink) => Some(Packet::Receive32 {
                                            step: receive::Step32::HeaderLength { sink },
                                            checksum: checksum
                                                .wrapping_add(bytes[2] as u16)
                                                .wrapping_add(bytes[3] as u16),
                                            attempt,
                                        }),
                                        Err((error, sink)) => Some(Packet::Receive32Error {
                                            step: receive::Step32Error::HeaderLength { sink },
                                            error: receive::Error::UnsupportedCommand(error),
                                            attempt,
                                        }),
                                    },
                                    Err(unknown) => Some(Packet::Receive32Error {
                                        step: receive::Step32Error::HeaderLength { sink },
                                        error: receive::Error::UnknownCommand(unknown),
                                        attempt,
                                    }),
                                },
                                byte => Some(Packet::Receive32Error {
                                    step: receive::Step32Error::HeaderLength { sink },
                                    error: receive::Error::MagicValue2(byte),
                                    attempt,
                                }),
                            },
                            byte => Some(Packet::Receive32Error {
                                step: receive::Step32Error::HeaderLength { sink },
                                error: receive::Error::MagicValue1(byte),
                                attempt,
                            }),
                        },
                        receive::Step32::HeaderLength { sink } => {
                            let full_length = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                            match sink.parse(full_length) {
                                Ok(Either::Left(sink)) => {
                                    // Receive the last two bytes as data.
                                    match sink.parse(bytes[2]) {
                                        Ok(Either::Left(sink)) => match sink.parse(bytes[3]) {
                                            Ok(Either::Left(sink)) => Some(Packet::Receive32 {
                                                step: receive::Step32::Data { sink },
                                                checksum: checksum
                                                    .wrapping_add(bytes[0] as u16)
                                                    .wrapping_add(bytes[1] as u16)
                                                    .wrapping_add(bytes[2] as u16)
                                                    .wrapping_add(bytes[3] as u16),
                                                attempt,
                                            }),
                                            Ok(Either::Right(result)) => Some(Packet::Receive32 {
                                                step: receive::Step32::Checksum { result },
                                                checksum: checksum
                                                    .wrapping_add(bytes[0] as u16)
                                                    .wrapping_add(bytes[1] as u16)
                                                    .wrapping_add(bytes[2] as u16)
                                                    .wrapping_add(bytes[3] as u16),
                                                attempt,
                                            }),
                                            Err((error, index, length, sink)) => {
                                                if let Some(next_index) = index.checked_add(1)
                                                    && next_index < length.get()
                                                {
                                                    // We still have more data to receive in the error state.
                                                    Some(Packet::Receive32Error {
                                                        step: receive::Step32Error::Data {
                                                            sink,
                                                            index: next_index,
                                                            length,
                                                        },
                                                        error: receive::Error::MalformedData(error),
                                                        attempt,
                                                    })
                                                } else {
                                                    // The error happened on the last byte being received.
                                                    Some(Packet::Receive32Error {
                                                        step: receive::Step32Error::Checksum {
                                                            sink,
                                                        },
                                                        error: receive::Error::MalformedData(error),
                                                        attempt,
                                                    })
                                                }
                                            }
                                        },
                                        Ok(Either::Right(result)) => Some(Packet::Receive32 {
                                            step: receive::Step32::Checksum { result },
                                            checksum: checksum
                                                .wrapping_add(bytes[0] as u16)
                                                .wrapping_add(bytes[1] as u16)
                                                .wrapping_add(bytes[2] as u16)
                                                .wrapping_add(bytes[3] as u16),
                                            attempt,
                                        }),
                                        Err((error, index, length, sink)) => {
                                            if let Some(next_index) = index.checked_add(2)
                                                && next_index < length.get()
                                            {
                                                // We still have more data to receive in the error state.
                                                Some(Packet::Receive32Error {
                                                    step: receive::Step32Error::Data {
                                                        sink,
                                                        index: next_index,
                                                        length,
                                                    },
                                                    error: receive::Error::MalformedData(error),
                                                    attempt,
                                                })
                                            } else {
                                                // The error happened on the last byte being received.
                                                Some(Packet::Receive32Error {
                                                    step: receive::Step32Error::Checksum { sink },
                                                    error: receive::Error::MalformedData(error),
                                                    attempt,
                                                })
                                            }
                                        }
                                    }
                                }
                                Ok(Either::Right(result)) => {
                                    // No data to receive, so we move right on to the checksum.
                                    let full_checksum =
                                        ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                                    if full_checksum == checksum {
                                        Some(Packet::Receive32 {
                                            step: receive::Step32::AcknowledgementSignal { result },
                                            checksum,
                                            attempt,
                                        })
                                    } else {
                                        Some(Packet::Receive32Error {
                                            step: receive::Step32Error::AcknowledgementSignal {
                                                sink: result.revert(),
                                            },
                                            error: receive::Error::Checksum {
                                                calculated: checksum,
                                                received: full_checksum,
                                            },
                                            attempt,
                                        })
                                    }
                                }
                                Err((error, sink)) => Some(Packet::Receive32Error {
                                    step: receive::Step32Error::AcknowledgementSignal { sink },
                                    error: receive::Error::UnexpectedLength(error),
                                    attempt,
                                }),
                            }
                        }
                        receive::Step32::Data { sink } => {
                            match sink.parse(bytes[0]) {
                                Ok(Either::Left(sink)) => {
                                    match sink.parse(bytes[1]) {
                                        Ok(Either::Left(sink)) => {
                                            match sink.parse(bytes[2]) {
                                                Ok(Either::Left(sink)) => {
                                                    match sink.parse(bytes[3]) {
                                                        Ok(Either::Left(sink)) => {
                                                            Some(Packet::Receive32 {
                                                                step: receive::Step32::Data {
                                                                    sink,
                                                                },
                                                                checksum: checksum
                                                                    .wrapping_add(bytes[0] as u16)
                                                                    .wrapping_add(bytes[1] as u16)
                                                                    .wrapping_add(bytes[2] as u16)
                                                                    .wrapping_add(bytes[3] as u16),
                                                                attempt,
                                                            })
                                                        }
                                                        Ok(Either::Right(result)) => {
                                                            Some(Packet::Receive32 {
                                                                step: receive::Step32::Checksum {
                                                                    result,
                                                                },
                                                                checksum: checksum
                                                                    .wrapping_add(bytes[0] as u16)
                                                                    .wrapping_add(bytes[1] as u16)
                                                                    .wrapping_add(bytes[2] as u16)
                                                                    .wrapping_add(bytes[3] as u16),
                                                                attempt,
                                                            })
                                                        }
                                                        Err((error, index, length, sink)) => {
                                                            if let Some(next_index) =
                                                                index.checked_add(1)
                                                                && next_index < length.get()
                                                            {
                                                                // We still have more data to receive in the error state.
                                                                Some(Packet::Receive32Error { step: receive::Step32Error::Data { sink, index: next_index, length }, error: receive::Error::MalformedData(error), attempt })
                                                            } else {
                                                                // The error happened on the last byte being received.
                                                                Some(Packet::Receive32Error { step: receive::Step32Error::Checksum { sink }, error: receive::Error::MalformedData(error), attempt })
                                                            }
                                                        }
                                                    }
                                                }
                                                Ok(Either::Right(result)) => {
                                                    Some(Packet::Receive32 {
                                                        step: receive::Step32::Checksum { result },
                                                        checksum: checksum
                                                            .wrapping_add(bytes[0] as u16)
                                                            .wrapping_add(bytes[1] as u16)
                                                            .wrapping_add(bytes[2] as u16)
                                                            .wrapping_add(bytes[3] as u16),
                                                        attempt,
                                                    })
                                                }
                                                Err((error, index, length, sink)) => {
                                                    if let Some(next_index) = index.checked_add(2)
                                                        && next_index < length.get()
                                                    {
                                                        // We still have more data to receive in the error state.
                                                        Some(Packet::Receive32Error {
                                                            step: receive::Step32Error::Data {
                                                                sink,
                                                                index: next_index,
                                                                length,
                                                            },
                                                            error: receive::Error::MalformedData(
                                                                error,
                                                            ),
                                                            attempt,
                                                        })
                                                    } else {
                                                        // The error happened on the last byte being received.
                                                        Some(Packet::Receive32Error {
                                                            step: receive::Step32Error::Checksum {
                                                                sink,
                                                            },
                                                            error: receive::Error::MalformedData(
                                                                error,
                                                            ),
                                                            attempt,
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                        Ok(Either::Right(result)) => {
                                            // The checksum is contained in the last two bytes.
                                            let calculated_checksum = checksum
                                                .wrapping_add(bytes[0] as u16)
                                                .wrapping_add(bytes[1] as u16);
                                            let full_checksum =
                                                ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                                            if full_checksum == calculated_checksum {
                                                Some(Packet::Receive32 {
                                                    step: receive::Step32::AcknowledgementSignal {
                                                        result,
                                                    },
                                                    checksum: calculated_checksum,
                                                    attempt,
                                                })
                                            } else {
                                                Some(Packet::Receive32Error { step: receive::Step32Error::AcknowledgementSignal { sink: result.revert() }, error: receive::Error::Checksum { calculated: calculated_checksum, received: full_checksum }, attempt })
                                            }
                                        }
                                        Err((error, index, length, sink)) => {
                                            if let Some(next_index) = index.checked_add(3)
                                                && next_index < length.get()
                                            {
                                                // We still have more data to receive in the error state.
                                                Some(Packet::Receive32Error {
                                                    step: receive::Step32Error::Data {
                                                        sink,
                                                        index: next_index,
                                                        length,
                                                    },
                                                    error: receive::Error::MalformedData(error),
                                                    attempt,
                                                })
                                            } else {
                                                // The error happened on the last byte being received.
                                                Some(Packet::Receive32Error { step: receive::Step32Error::AcknowledgementSignal { sink }, error: receive::Error::MalformedData(error), attempt })
                                            }
                                        }
                                    }
                                }
                                Ok(Either::Right(result)) => {
                                    // The checksum is contained in the last two bytes.
                                    let calculated_checksum = checksum
                                        .wrapping_add(bytes[0] as u16)
                                        .wrapping_add(bytes[1] as u16);
                                    let full_checksum =
                                        ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                                    if full_checksum == calculated_checksum {
                                        Some(Packet::Receive32 {
                                            step: receive::Step32::AcknowledgementSignal { result },
                                            checksum: calculated_checksum,
                                            attempt,
                                        })
                                    } else {
                                        Some(Packet::Receive32Error {
                                            step: receive::Step32Error::AcknowledgementSignal {
                                                sink: result.revert(),
                                            },
                                            error: receive::Error::Checksum {
                                                calculated: calculated_checksum,
                                                received: full_checksum,
                                            },
                                            attempt,
                                        })
                                    }
                                }
                                Err((error, index, length, sink)) => {
                                    if let Some(next_index) = index.checked_add(4)
                                        && next_index < length.get()
                                    {
                                        // We still have more data to receive in the error state.
                                        Some(Packet::Receive32Error {
                                            step: receive::Step32Error::Data {
                                                sink,
                                                index: next_index,
                                                length,
                                            },
                                            error: receive::Error::MalformedData(error),
                                            attempt,
                                        })
                                    } else {
                                        // The error happened on the last byte being received.
                                        Some(Packet::Receive32Error {
                                            step: receive::Step32Error::AcknowledgementSignal {
                                                sink,
                                            },
                                            error: receive::Error::MalformedData(error),
                                            attempt,
                                        })
                                    }
                                }
                            }
                        }
                        receive::Step32::Checksum { result } => {
                            // The checksum is contained in the last two bytes.
                            let calculated_checksum = checksum
                                .wrapping_add(bytes[0] as u16)
                                .wrapping_add(bytes[1] as u16);
                            let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                            if full_checksum == calculated_checksum {
                                Some(Packet::Receive32 {
                                    step: receive::Step32::AcknowledgementSignal { result },
                                    checksum: calculated_checksum,
                                    attempt,
                                })
                            } else {
                                Some(Packet::Receive32Error {
                                    step: receive::Step32Error::AcknowledgementSignal {
                                        sink: result.revert(),
                                    },
                                    error: receive::Error::Checksum {
                                        calculated: calculated_checksum,
                                        received: full_checksum,
                                    },
                                    attempt,
                                })
                            }
                        }
                        receive::Step32::AcknowledgementSignal { result } => {
                            match Adapter::try_from(bytes[0]) {
                                Ok(received_adapter) => match NonZeroU8::new(bytes[1]) {
                                    None => {
                                        // We don't care about what the adapter was set to previously.
                                        // We just want to store whatever type it's currently telling
                                        // us it is.
                                        *adapter = received_adapter;
                                        match result.finish() {
                                            sink::Finished::Success => {
                                                if let Some(new_flow) = flow.next() {
                                                    *flow = new_flow;
                                                } else {
                                                    self.state = State::P2P;
                                                    return;
                                                }
                                            }
                                        }
                                        None
                                    }
                                    Some(nonzero) => {
                                        // We can no longer retry at this point. We simply enter an
                                        // error state.
                                        self.state = State::Error(Error::Packet(packet::Error::Receive(packet::receive::Error::NonZeroAcknowledgementCommand(nonzero))));
                                        return;
                                    }
                                },
                                Err(unknown) => {
                                    // We can no longer retry at this point. We simply enter an
                                    // error state.
                                    self.state =
                                        State::Error(Error::Packet(packet::Error::Receive(
                                            packet::receive::Error::UnsupportedDevice(unknown),
                                        )));
                                    return;
                                }
                            }
                        }
                    }
                }

                // /--------------------\
                // | SIO8 Receive Error |
                // \--------------------/
                Packet::Receive8Error {
                    step,
                    error,
                    attempt,
                } => {
                    let byte = unsafe { SIODATA8.read_volatile() };
                    match step {
                        receive::Step8Error::MagicByte2 { sink } => Some(Packet::Receive8Error {
                            step: receive::Step8Error::HeaderCommand { sink },
                            error,
                            attempt,
                        }),
                        receive::Step8Error::HeaderCommand { sink } => {
                            Some(Packet::Receive8Error {
                                step: receive::Step8Error::HeaderEmptyByte { sink },
                                error,
                                attempt,
                            })
                        }
                        receive::Step8Error::HeaderEmptyByte { sink } => {
                            Some(Packet::Receive8Error {
                                step: receive::Step8Error::HeaderLength1 { sink },
                                error,
                                attempt,
                            })
                        }
                        receive::Step8Error::HeaderLength1 { sink } => {
                            Some(Packet::Receive8Error {
                                step: receive::Step8Error::HeaderLength2 {
                                    sink,
                                    first_byte: byte,
                                },
                                error,
                                attempt,
                            })
                        }
                        receive::Step8Error::HeaderLength2 { sink, first_byte } => {
                            let full_length = ((first_byte as u16) << 8) | (byte as u16);
                            match NonZeroU16::new(full_length) {
                                Some(length) => Some(Packet::Receive8Error {
                                    step: receive::Step8Error::Data {
                                        sink,
                                        index: 0,
                                        length,
                                    },
                                    error,
                                    attempt,
                                }),
                                None => Some(Packet::Receive8Error {
                                    step: receive::Step8Error::Checksum1 { sink },
                                    error,
                                    attempt,
                                }),
                            }
                        }
                        receive::Step8Error::Data {
                            sink,
                            index,
                            length,
                        } => {
                            let next_index = index + 1;
                            if next_index < length.get() {
                                Some(Packet::Receive8Error {
                                    step: receive::Step8Error::Data {
                                        sink,
                                        index: next_index,
                                        length,
                                    },
                                    error,
                                    attempt,
                                })
                            } else {
                                Some(Packet::Receive8Error {
                                    step: receive::Step8Error::Checksum1 { sink },
                                    error,
                                    attempt,
                                })
                            }
                        }
                        receive::Step8Error::Checksum1 { sink } => Some(Packet::Receive8Error {
                            step: receive::Step8Error::Checksum2 { sink },
                            error,
                            attempt,
                        }),
                        receive::Step8Error::Checksum2 { sink } => Some(Packet::Receive8Error {
                            step: receive::Step8Error::AcknowledgementSignalDevice { sink },
                            error,
                            attempt,
                        }),
                        receive::Step8Error::AcknowledgementSignalDevice { sink } => {
                            Some(Packet::Receive8Error {
                                step: receive::Step8Error::AcknowledgementSignalCommand { sink },
                                error,
                                attempt,
                            })
                        }
                        receive::Step8Error::AcknowledgementSignalCommand { sink } => {
                            let new_attempt = attempt + 1;
                            if new_attempt < packet::MAX_RETRIES {
                                // Retry.
                                Some(Packet::Receive8 {
                                    step: receive::Step8::MagicByte1 { sink },
                                    checksum: 0,
                                    attempt: new_attempt,
                                })
                            } else {
                                // Too many retries. Stop trying and set error state.
                                self.state =
                                    State::Error(Error::Packet(packet::Error::Receive(error)));
                                return;
                            }
                        }
                    }
                }

                // /---------------------\
                // | SIO32 Receive Error |
                // \---------------------/
                Packet::Receive32Error {
                    step,
                    error,
                    attempt,
                } => {
                    let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };
                    match step {
                        receive::Step32Error::HeaderLength { sink } => {
                            let full_length = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                            match NonZeroU16::new(full_length) {
                                Some(length) => {
                                    if 2 < length.get() {
                                        Some(Packet::Receive32Error {
                                            step: receive::Step32Error::Checksum { sink },
                                            error,
                                            attempt,
                                        })
                                    } else {
                                        Some(Packet::Receive32Error {
                                            step: receive::Step32Error::Data {
                                                sink,
                                                index: 2,
                                                length,
                                            },
                                            error,
                                            attempt,
                                        })
                                    }
                                }
                                None => Some(Packet::Receive32Error {
                                    step: receive::Step32Error::AcknowledgementSignal { sink },
                                    error,
                                    attempt,
                                }),
                            }
                        }
                        receive::Step32Error::Data {
                            sink,
                            index,
                            length,
                        } => {
                            if index + 2 >= length.get() {
                                // Checksum is included in last two bytes.
                                Some(Packet::Receive32Error {
                                    step: receive::Step32Error::AcknowledgementSignal { sink },
                                    error,
                                    attempt,
                                })
                            } else if index + 4 >= length.get() {
                                // These are the last data bytes.
                                Some(Packet::Receive32Error {
                                    step: receive::Step32Error::Checksum { sink },
                                    error,
                                    attempt,
                                })
                            } else {
                                // There is more data.
                                Some(Packet::Receive32Error {
                                    step: receive::Step32Error::Data {
                                        sink,
                                        index: index + 4,
                                        length,
                                    },
                                    error,
                                    attempt,
                                })
                            }
                        }
                        receive::Step32Error::Checksum { sink } => Some(Packet::Receive32Error {
                            step: receive::Step32Error::AcknowledgementSignal { sink },
                            error,
                            attempt,
                        }),
                        receive::Step32Error::AcknowledgementSignal { sink } => {
                            let new_attempt = attempt + 1;
                            if new_attempt < packet::MAX_RETRIES {
                                // Retry.
                                Some(Packet::Receive32 {
                                    step: receive::Step32::MagicByte { sink },
                                    checksum: 0,
                                    attempt: new_attempt,
                                })
                            } else {
                                // Too many retries. Stop trying and set error state.
                                self.state =
                                    State::Error(Error::Packet(packet::Error::Receive(error)));
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::{
        engine::{Adapter, Command, HANDSHAKE, command},
        mmio::serial::{self, Mode, RCNT, SIOCNT, SIODATA8, SIODATA32, TransferLength},
    };
    use gba_test::test;

    macro_rules! assert_sio_8 {
        ($engine:ident, $send:expr, $receive:expr $(,)?) => {
            $engine.timer();
            assert_eq!(unsafe { SIODATA8.read_volatile() }, $send);
            unsafe { SIODATA8.write_volatile($receive) };
            $engine.serial();
        };
    }

    macro_rules! assert_sio_32 {
        ($engine:ident, $send:expr, $receive:expr $(,)?) => {
            $engine.timer();
            assert_eq!(unsafe { SIODATA32.read_volatile() }, $send);
            unsafe { SIODATA32.write_volatile($receive) };
            $engine.serial();
        };
    }

    #[test]
    fn begin_session_send8() {
        // Enter Normal SIO8 mode so that SIODATA can be used.
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(TransferLength::_8Bit));
        }

        let mut engine = Engine::new();
        engine.state = super::State::LinkingP2P {
            adapter: super::Adapter::Blue,
            transfer_length: TransferLength::_8Bit,
            packet: None,
            flow: super::flow::LinkingP2P::BeginSession,
        };
        engine.vblank();

        // Magic values.
        assert_sio_8!(engine, 0x99, 0xd2);
        assert_sio_8!(engine, 0x66, 0xd2);

        // Header.
        // Command.
        assert_sio_8!(engine, Command::BeginSession as u8, 0xd2);
        assert_sio_8!(engine, 0x00, 0xd2);
        // Length.
        assert_sio_8!(engine, 0x00, 0xd2);
        assert_sio_8!(engine, 0x08, 0xd2);

        // Data.
        assert_sio_8!(engine, HANDSHAKE[0], 0xd2);
        assert_sio_8!(engine, HANDSHAKE[1], 0xd2);
        assert_sio_8!(engine, HANDSHAKE[2], 0xd2);
        assert_sio_8!(engine, HANDSHAKE[3], 0xd2);
        assert_sio_8!(engine, HANDSHAKE[4], 0xd2);
        assert_sio_8!(engine, HANDSHAKE[5], 0xd2);
        assert_sio_8!(engine, HANDSHAKE[6], 0xd2);
        assert_sio_8!(engine, HANDSHAKE[7], 0xd2);

        // Checksum.
        assert_sio_8!(engine, 0x02, 0xd2);
        assert_sio_8!(engine, 0x77, 0xd2);

        // Acknowledgement Signal.
        assert_sio_8!(engine, 0x81, Adapter::Blue as u8);
        assert_sio_8!(engine, 0x00, Command::BeginSession as u8 ^ 0x80);
    }

    #[test]
    fn begin_session_send32() {
        // Enter Normal SIO32 mode so that SIODATA can be used.
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(TransferLength::_32Bit));
        }

        let mut engine = Engine::new();
        engine.state = super::State::LinkingP2P {
            adapter: super::Adapter::Blue,
            transfer_length: TransferLength::_32Bit,
            packet: None,
            flow: super::flow::LinkingP2P::BeginSession,
        };
        engine.vblank();

        // Magic values + command.
        assert_sio_32!(
            engine,
            u32::from_be_bytes([0x99, 0x66, Command::BeginSession as u8, 0x00]),
            0xd2_d2_d2_d2,
        );

        // Length + Data.
        assert_sio_32!(
            engine,
            u32::from_be_bytes([0x00, 0x08, HANDSHAKE[0], HANDSHAKE[1]]),
            0xd2_d2_d2_d2,
        );
        assert_sio_32!(
            engine,
            u32::from_be_bytes([HANDSHAKE[2], HANDSHAKE[3], HANDSHAKE[4], HANDSHAKE[5]]),
            0xd2_d2_d2_d2,
        );
        // Data + Checksum
        assert_sio_32!(
            engine,
            u32::from_be_bytes([HANDSHAKE[6], HANDSHAKE[7], 0x02, 0x77]),
            0xd2_d2_d2_d2,
        );

        // Acknowledgement Signal.
        assert_sio_32!(
            engine,
            u32::from_be_bytes([0x81, 0x00, 0x00, 0x00]),
            u32::from_be_bytes([
                Adapter::Blue as u8,
                Command::BeginSession as u8 ^ 0x80,
                0x00,
                0x00,
            ]),
        );
    }

    #[test]
    fn begin_session_receive8() {
        // Enter Normal SIO8 mode so that SIODATA can be used.
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(TransferLength::_8Bit));
        }

        let mut engine = Engine::new();
        engine.state = super::State::LinkingP2P {
            adapter: super::Adapter::Blue,
            transfer_length: TransferLength::_8Bit,

            packet: Some(super::Packet::Receive8 {
                step: super::receive::Step8::MagicByte1 {
                    sink: super::sink::Command::BeginSession,
                },
                checksum: 0,
                attempt: 0,
            }),
            flow: super::flow::LinkingP2P::BeginSession,
        };
        engine.vblank();

        // Magic values.
        assert_sio_8!(engine, 0x4b, 0x99);
        assert_sio_8!(engine, 0x4b, 0x66);

        // Header.
        // Command.
        assert_sio_8!(engine, 0x4b, Command::BeginSession as u8);
        assert_sio_8!(engine, 0x4b, 0x00);
        // Length.
        assert_sio_8!(engine, 0x4b, 0x00);
        assert_sio_8!(engine, 0x4b, 0x08);

        // Data.
        assert_sio_8!(engine, 0x4b, HANDSHAKE[0]);
        assert_sio_8!(engine, 0x4b, HANDSHAKE[1]);
        assert_sio_8!(engine, 0x4b, HANDSHAKE[2]);
        assert_sio_8!(engine, 0x4b, HANDSHAKE[3]);
        assert_sio_8!(engine, 0x4b, HANDSHAKE[4]);
        assert_sio_8!(engine, 0x4b, HANDSHAKE[5]);
        assert_sio_8!(engine, 0x4b, HANDSHAKE[6]);
        assert_sio_8!(engine, 0x4b, HANDSHAKE[7]);

        // Checksum.
        assert_sio_8!(engine, 0x4b, 0x02);
        assert_sio_8!(engine, 0x4b, 0x77);

        // Acknowledgement Signal.
        assert_sio_8!(engine, 0x81, Adapter::Blue as u8);
        assert_sio_8!(engine, Command::BeginSession as u8 ^ 0x80, 0x00);
    }

    #[test]
    fn begin_session_receive8_command_error() {
        // Enter Normal SIO8 mode so that SIODATA can be used.
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(TransferLength::_8Bit));
        }

        let mut engine = Engine::new();
        engine.state = super::State::LinkingP2P {
            adapter: super::Adapter::Blue,
            transfer_length: TransferLength::_8Bit,

            packet: Some(super::Packet::Receive8 {
                step: super::receive::Step8::MagicByte1 {
                    sink: super::sink::Command::BeginSession,
                },
                checksum: 0,
                attempt: 0,
            }),
            flow: super::flow::LinkingP2P::BeginSession,
        };
        engine.vblank();

        // Magic values.
        assert_sio_8!(engine, 0x4b, 0x99);
        assert_sio_8!(engine, 0x4b, 0x66);

        // Header.
        // Command.
        assert_sio_8!(engine, 0x4b, Command::CommandError as u8);
        assert_sio_8!(engine, 0x4b, 0x00);
        // Length.
        assert_sio_8!(engine, 0x4b, 0x00);
        assert_sio_8!(engine, 0x4b, 0x02);

        // Data.
        assert_sio_8!(engine, 0x4b, Command::BeginSession as u8);
        assert_sio_8!(
            engine,
            0x4b,
            command::error::begin_session::Error::AlreadyActive as u8
        );

        // Checksum.
        assert_sio_8!(engine, 0x4b, 0x00);
        assert_sio_8!(engine, 0x4b, 0x81);

        // Acknowledgement Signal.
        assert_sio_8!(engine, 0x81, Adapter::Blue as u8);
        assert_sio_8!(engine, Command::CommandError as u8 ^ 0x80, 0x00);
    }

    #[test]
    fn begin_session_receive32() {
        // Enter Normal SIO8 mode so that SIODATA can be used.
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(TransferLength::_32Bit));
        }

        let mut engine = Engine::new();
        engine.state = super::State::LinkingP2P {
            adapter: super::Adapter::Blue,
            transfer_length: TransferLength::_32Bit,

            packet: Some(super::Packet::Receive32 {
                step: super::receive::Step32::MagicByte {
                    sink: super::sink::Command::BeginSession,
                },
                checksum: 0,
                attempt: 0,
            }),
            flow: super::flow::LinkingP2P::BeginSession,
        };
        engine.vblank();

        // Magic values + command.
        assert_sio_32!(
            engine,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([0x99, 0x66, Command::BeginSession as u8, 0x00])
        );

        // Length + Data.
        assert_sio_32!(
            engine,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([0x00, 0x08, HANDSHAKE[0], HANDSHAKE[1]]),
        );
        assert_sio_32!(
            engine,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([HANDSHAKE[2], HANDSHAKE[3], HANDSHAKE[4], HANDSHAKE[5]]),
        );
        // Data + Checksum
        assert_sio_32!(
            engine,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([HANDSHAKE[6], HANDSHAKE[7], 0x02, 0x77]),
        );

        // Acknowledgement Signal.
        assert_sio_32!(
            engine,
            u32::from_be_bytes([0x81, Command::BeginSession as u8 ^ 0x80, 0x00, 0x00]),
            u32::from_be_bytes([Adapter::Blue as u8, 0x00, 0x00, 0x00]),
        );
    }

    #[test]
    fn begin_session_receive32_command_error() {
        // Enter Normal SIO8 mode so that SIODATA can be used.
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(TransferLength::_32Bit));
        }

        let mut engine = Engine::new();
        engine.state = super::State::LinkingP2P {
            adapter: super::Adapter::Blue,
            transfer_length: TransferLength::_32Bit,

            packet: Some(super::Packet::Receive32 {
                step: super::receive::Step32::MagicByte {
                    sink: super::sink::Command::BeginSession,
                },
                checksum: 0,
                attempt: 0,
            }),
            flow: super::flow::LinkingP2P::BeginSession,
        };
        engine.vblank();

        // Magic values + command.
        assert_sio_32!(
            engine,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([0x99, 0x66, Command::CommandError as u8, 0x00])
        );

        // Length + Data.
        assert_sio_32!(
            engine,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([
                0x00,
                0x02,
                Command::BeginSession as u8,
                command::error::begin_session::Error::AlreadyActive as u8
            ]),
        );
        // Checksum
        assert_sio_32!(
            engine,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([0x00, 0x00, 0x00, 0x81]),
        );

        // Acknowledgement Signal.
        assert_sio_32!(
            engine,
            u32::from_be_bytes([0x81, Command::CommandError as u8 ^ 0x80, 0x00, 0x00]),
            u32::from_be_bytes([Adapter::Blue as u8, 0x00, 0x00, 0x00]),
        );
    }
}
