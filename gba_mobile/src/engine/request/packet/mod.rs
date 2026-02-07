pub(in crate::engine) mod receive;
pub(in crate::engine) mod send;

mod error;
mod timeout;

pub(in crate::engine) use error::Error;
pub(in crate::engine) use timeout::Timeout;

use super::FRAMES_3_SECONDS;
use crate::{
    engine::{Adapter, Command, Source, command, sink},
    mmio::serial::{SIOCNT, SIODATA8, SIODATA32, TransferLength},
};
use core::num::{NonZeroU8, NonZeroU16};
use either::Either;

pub(in crate::engine) const MAX_RETRIES: u8 = 5;

const FRAMES_15_SECONDS: u16 = 900;

/// In-progress communication.
#[derive(Debug)]
pub(in crate::engine) enum Packet {
    /// Sending in SIO8 mode.
    Send8 {
        step: send::Step8,
        source: Source,
        checksum: u16,

        attempt: u8,
        frame: u8,
    },
    /// Sending in SIO32 mode.
    Send32 {
        step: send::Step32,
        source: Source,
        checksum: u16,

        attempt: u8,
        frame: u8,
    },
    /// Receiving in SIO8 mode.
    Receive8 {
        step: receive::Step8,
        checksum: u16,

        attempt: u8,
        frame: u8,
    },
    /// Receiving in SIO32 mode.
    Receive32 {
        step: receive::Step32,
        checksum: u16,

        attempt: u8,
        frame: u8,
    },
    /// Receiving in SIO8 mode while in an error state.
    Receive8Error {
        step: receive::Step8Error,

        error: receive::Error,
        attempt: u8,
        frame: u8,
    },
    /// Receiving in SIO32 mode while in an error state.
    Receive32Error {
        step: receive::Step32Error,

        error: receive::Error,
        attempt: u8,
        frame: u8,
    },
}

impl Packet {
    pub(in crate::engine) fn new(transfer_length: TransferLength, source: Source) -> Self {
        match transfer_length {
            TransferLength::_8Bit => Self::Send8 {
                step: send::Step8::MagicByte1,
                source,
                checksum: 0,
                attempt: 0,
                frame: 0,
            },
            TransferLength::_32Bit => Self::Send32 {
                step: send::Step32::MagicByte,
                source,
                checksum: 0,
                attempt: 0,
                frame: 0,
            },
        }
    }

    /// Increment the frame count and return a timeout if one has occurred.
    pub(in crate::engine) fn timeout(&mut self) -> Result<(), Timeout> {
        let (Self::Send8 { frame, .. }
        | Self::Send32 { frame, .. }
        | Self::Receive8 { frame, .. }
        | Self::Receive32 { frame, .. }
        | Self::Receive8Error { frame, .. }
        | Self::Receive32Error { frame, .. }) = self;
        if *frame > FRAMES_3_SECONDS {
            return Err(Timeout::Serial);
        } else {
            *frame += 1;
        }

        if let Self::Receive8 {
            step: receive::Step8::MagicByte1 {
                frame: step_frame, ..
            },
            ..
        }
        | Self::Receive32 {
            step: receive::Step32::MagicByte {
                frame: step_frame, ..
            },
            ..
        } = self
        {
            if *step_frame > FRAMES_15_SECONDS {
                return Err(Timeout::Response);
            }
            *step_frame += 1;
        }

        Ok(())
    }

    /// Push bytes into SIO from this packet.
    pub(in crate::engine) fn push(&mut self) {
        match self {
            // /-----------\
            // | SIO8 Send |
            // \-----------/
            Self::Send8 {
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
            Self::Send32 {
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
                    send::Step32::Checksum => {
                        u32::from_be_bytes([0x00, 0x00, (*checksum >> 8) as u8, *checksum as u8])
                    }
                    send::Step32::AcknowledgementSignal => 0x81_00_00_00,
                };

                unsafe { SIODATA32.write_volatile(bytes) };
            }

            // /--------------\
            // | SIO8 Receive |
            // \--------------/
            Self::Receive8 { step, .. } => {
                let byte = match step {
                    receive::Step8::AcknowledgementSignalDevice { .. } => 0x81,
                    receive::Step8::AcknowledgementSignalCommand {
                        result,
                        command_xor,
                        ..
                    } => {
                        if *command_xor {
                            result.command() as u8 | 0x80
                        } else {
                            result.command() as u8
                        }
                    }
                    _ => 0x4b,
                };
                unsafe { SIODATA8.write_volatile(byte) };
            }

            // /---------------\
            // | SIO32 Receive |
            // \---------------/
            Self::Receive32 { step, .. } => {
                let bytes = match step {
                    receive::Step32::AcknowledgementSignal {
                        result,
                        command_xor,
                        ..
                    } => {
                        let command_byte = if *command_xor {
                            result.command() as u8 | 0x80
                        } else {
                            result.command() as u8
                        };
                        u32::from_be_bytes([0x81, command_byte, 0x00, 0x00])
                    }
                    _ => 0x4b_4b_4b_4b,
                };
                unsafe { SIODATA32.write_volatile(bytes) };
            }

            // /--------------------\
            // | SIO8 Receive Error |
            // \--------------------/
            Self::Receive8Error {
                step,
                error,
                attempt,
                ..
            } => {
                let byte = match step {
                    receive::Step8Error::AcknowledgementSignalDevice { .. } => 0x81,
                    receive::Step8Error::AcknowledgementSignalCommand { .. } => {
                        if *attempt + 1 < MAX_RETRIES {
                            error.command() as u8 | 0x80
                        } else {
                            // Since we've errored on communication too much, it doesn't matter
                            // what we send here. We are going to error out the link session
                            // anyway.
                            Command::Empty as u8 | 0x80
                        }
                    }
                    _ => 0x4b,
                };
                unsafe { SIODATA8.write_volatile(byte) };
            }

            // /---------------------\
            // | SIO32 Receive Error |
            // \---------------------/
            Self::Receive32Error {
                step,
                error,
                attempt,
                ..
            } => {
                let bytes = match step {
                    receive::Step32Error::AcknowledgementSignal { .. } => {
                        let command_byte = if *attempt + 1 < MAX_RETRIES {
                            error.command() as u8 | 0x80
                        } else {
                            // Since we've errored on communication too much, it doesn't matter
                            // what we send here. We are going to error out the link session
                            // anyway.
                            Command::Empty as u8 | 0x80
                        };
                        u32::from_be_bytes([0x81, command_byte, 0x00, 0x00])
                    }
                    _ => 0x4b_4b_4b_4b,
                };
                unsafe { SIODATA32.write_volatile(bytes) };
            }
        }
    }

    /// Pull bytes from SIO into this packet.
    pub(in crate::engine) fn pull(
        self,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
    ) -> Result<Option<Self>, Either<Error, command::Error>> {
        match self {
            // /-----------\
            // | SIO8 Send |
            // \-----------/
            Self::Send8 {
                step,
                source,
                checksum,
                attempt,
                ..
            } => {
                let byte = unsafe { SIODATA8.read_volatile() };
                log::debug!("received byte {byte:#04x}");
                match step {
                    send::Step8::MagicByte1 => Ok(Some(Self::Send8 {
                        step: send::Step8::MagicByte2,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step8::MagicByte2 => Ok(Some(Self::Send8 {
                        step: send::Step8::HeaderCommand,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step8::HeaderCommand => Ok(Some(Self::Send8 {
                        step: send::Step8::HeaderEmptyByte,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step8::HeaderEmptyByte => Ok(Some(Self::Send8 {
                        step: send::Step8::HeaderLength1,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step8::HeaderLength1 => Ok(Some(Self::Send8 {
                        step: send::Step8::HeaderLength2,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step8::HeaderLength2 => {
                        if source.length() > 0 {
                            Ok(Some(Self::Send8 {
                                step: send::Step8::Data { index: 0 },
                                source,
                                checksum,
                                attempt,
                                frame: 0,
                            }))
                        } else {
                            Ok(Some(Self::Send8 {
                                step: send::Step8::Checksum1,
                                source,
                                checksum,
                                attempt,
                                frame: 0,
                            }))
                        }
                    }
                    send::Step8::Data { index } => {
                        let next_index = index + 1;
                        if source.length() > next_index {
                            Ok(Some(Self::Send8 {
                                step: send::Step8::Data { index: next_index },
                                source,
                                checksum,
                                attempt,
                                frame: 0,
                            }))
                        } else {
                            Ok(Some(Self::Send8 {
                                step: send::Step8::Checksum1,
                                source,
                                checksum,
                                attempt,
                                frame: 0,
                            }))
                        }
                    }
                    send::Step8::Checksum1 => Ok(Some(Self::Send8 {
                        step: send::Step8::Checksum2,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step8::Checksum2 { .. } => Ok(Some(Self::Send8 {
                        step: send::Step8::AcknowledgementSignalDevice,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step8::AcknowledgementSignalDevice => Ok(Some(Self::Send8 {
                        step: send::Step8::AcknowledgementSignalCommand,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step8::AcknowledgementSignalCommand => {
                        let new_attempt = attempt + 1;
                        match Command::try_from(byte ^ 0x80) {
                            Ok(
                                Command::NotSupportedError
                                | Command::MalformedError
                                | Command::InternalError,
                            ) if new_attempt < MAX_RETRIES => {
                                // Retry.
                                Ok(Some(Self::Send8 {
                                    step: send::Step8::MagicByte1,
                                    source,
                                    checksum: 0,
                                    attempt: new_attempt,
                                    frame: 0,
                                }))
                            }
                            Ok(Command::NotSupportedError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(Either::Left(Error::Send(send::Error::UnsupportedCommand(
                                    source.command(),
                                ))))
                            }
                            Ok(Command::MalformedError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(Either::Left(Error::Send(send::Error::Malformed)))
                            }
                            Ok(Command::InternalError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(Either::Left(Error::Send(send::Error::AdapterInternalError)))
                            }
                            _ => {
                                // We don't verify anything here and simply assume the adapter
                                // responded with a correct command. If the adapter is in an invalid
                                // state, we will find out when receiving the response packet instead.
                                Ok(Some(Self::Receive8 {
                                    step: receive::Step8::MagicByte1 {
                                        sink: source.sink(),
                                        frame: 0,
                                    },
                                    checksum: 0,

                                    attempt: 0,
                                    frame: 0,
                                }))
                            }
                        }
                    }
                }
            }

            // /------------\
            // | SIO32 Send |
            // \------------/
            Self::Send32 {
                step,
                source,
                checksum,
                attempt,
                ..
            } => {
                let bytes = unsafe { SIODATA32.read_volatile() };
                match step {
                    send::Step32::MagicByte => Ok(Some(Self::Send32 {
                        step: send::Step32::HeaderLength,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step32::HeaderLength => {
                        let new_step = match source.length() {
                            0 => send::Step32::AcknowledgementSignal,
                            1..=2 => send::Step32::Checksum,
                            _ => send::Step32::Data { index: 2 },
                        };
                        Ok(Some(Self::Send32 {
                            step: new_step,
                            source,
                            checksum,
                            attempt,
                            frame: 0,
                        }))
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
                        Ok(Some(Self::Send32 {
                            step: new_step,
                            source,
                            checksum,
                            attempt,
                            frame: 0,
                        }))
                    }
                    send::Step32::Checksum => Ok(Some(Self::Send32 {
                        step: send::Step32::AcknowledgementSignal,
                        source,
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    send::Step32::AcknowledgementSignal => {
                        let new_attempt = attempt + 1;
                        match Command::try_from(bytes.to_be_bytes()[1] ^ 0x80) {
                            Ok(
                                Command::NotSupportedError
                                | Command::MalformedError
                                | Command::InternalError,
                            ) if new_attempt < MAX_RETRIES => {
                                // Retry.
                                Ok(Some(Self::Send32 {
                                    step: send::Step32::MagicByte,
                                    source,
                                    checksum: 0,
                                    attempt: new_attempt,
                                    frame: 0,
                                }))
                            }
                            Ok(Command::NotSupportedError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(Either::Left(Error::Send(send::Error::UnsupportedCommand(
                                    source.command(),
                                ))))
                            }
                            Ok(Command::MalformedError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(Either::Left(Error::Send(send::Error::Malformed)))
                            }
                            Ok(Command::InternalError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(Either::Left(Error::Send(send::Error::AdapterInternalError)))
                            }
                            _ => {
                                // We don't verify anything here and simply assume the adapter
                                // responded with a correct command. If the adapter is in an invalid
                                // state, we will find out when receiving the response packet instead.
                                Ok(Some(Self::Receive32 {
                                    step: receive::Step32::MagicByte {
                                        sink: source.sink(),
                                        frame: 0,
                                    },
                                    checksum: 0,

                                    attempt: 0,
                                    frame: 0,
                                }))
                            }
                        }
                    }
                }
            }

            // /--------------\
            // | SIO8 Receive |
            // \--------------/
            Self::Receive8 {
                step,
                checksum,
                attempt,
                ..
            } => {
                let byte = unsafe { SIODATA8.read_volatile() };
                log::debug!("received byte {byte:#04x}");
                match step {
                    receive::Step8::MagicByte1 {
                        sink,
                        frame: step_frame,
                    } => match byte {
                        0x99 => Ok(Some(Self::Receive8 {
                            step: receive::Step8::MagicByte2 { sink },
                            checksum,
                            attempt,
                            frame: 0,
                        })),
                        0xd2 => Ok(Some(Self::Receive8 {
                            step: receive::Step8::MagicByte1 {
                                sink,
                                frame: step_frame,
                            },
                            checksum,
                            attempt,
                            frame: 0,
                        })),
                        _ => Ok(Some(Self::Receive8Error {
                            step: receive::Step8Error::MagicByte2 { sink },
                            error: receive::Error::MagicValue1(byte),
                            attempt,
                            frame: 0,
                        })),
                    },
                    receive::Step8::MagicByte2 { sink } => match byte {
                        0x66 => Ok(Some(Self::Receive8 {
                            step: receive::Step8::HeaderCommand { sink },
                            checksum,
                            attempt,
                            frame: 0,
                        })),
                        _ => Ok(Some(Self::Receive8Error {
                            step: receive::Step8Error::HeaderCommand { sink },
                            error: receive::Error::MagicValue2(byte),
                            attempt,
                            frame: 0,
                        })),
                    },
                    // TODO: This probably needs to accept either 0x80 or no 0x80. Then we need to take whatever this is and XOR it with 0x80 at the end again?
                    receive::Step8::HeaderCommand { sink } => {
                        let command_xor = byte & 0x80 == 0;
                        match Command::try_from(byte & 0x7F) {
                            Ok(command) => match sink.parse(command) {
                                Ok(sink) => Ok(Some(Self::Receive8 {
                                    step: receive::Step8::HeaderEmptyByte { sink, command_xor },
                                    checksum: checksum.wrapping_add(byte as u16),
                                    attempt,
                                    frame: 0,
                                })),
                                Err((error, sink)) => Ok(Some(Self::Receive8Error {
                                    step: receive::Step8Error::HeaderEmptyByte { sink },
                                    error: receive::Error::UnsupportedCommand(error),
                                    attempt,
                                    frame: 0,
                                })),
                            },
                            Err(unknown) => Ok(Some(Self::Receive8Error {
                                step: receive::Step8Error::HeaderEmptyByte { sink },
                                error: receive::Error::UnknownCommand(unknown),
                                attempt,
                                frame: 0,
                            })),
                        }
                    }
                    receive::Step8::HeaderEmptyByte { sink, command_xor } => {
                        Ok(Some(Self::Receive8 {
                            step: receive::Step8::HeaderLength1 { sink, command_xor },
                            checksum: checksum.wrapping_add(byte as u16),
                            attempt,
                            frame: 0,
                        }))
                    }
                    receive::Step8::HeaderLength1 { sink, command_xor } => {
                        Ok(Some(Self::Receive8 {
                            step: receive::Step8::HeaderLength2 {
                                first_byte: byte,
                                sink,
                                command_xor,
                            },
                            checksum: checksum.wrapping_add(byte as u16),
                            attempt,
                            frame: 0,
                        }))
                    }
                    receive::Step8::HeaderLength2 {
                        sink,
                        first_byte,
                        command_xor,
                    } => {
                        let full_length = ((first_byte as u16) << 8) | (byte as u16);
                        match sink.parse(full_length) {
                            Ok(Either::Left(sink)) => Ok(Some(Self::Receive8 {
                                step: receive::Step8::Data { sink, command_xor },
                                checksum: checksum.wrapping_add(byte as u16),
                                attempt,
                                frame: 0,
                            })),
                            Ok(Either::Right(result)) => Ok(Some(Self::Receive8 {
                                step: receive::Step8::Checksum1 {
                                    result,
                                    command_xor,
                                },
                                checksum: checksum.wrapping_add(byte as u16),
                                attempt,
                                frame: 0,
                            })),
                            Err((error, sink)) => match NonZeroU16::new(full_length) {
                                Some(length) => Ok(Some(Self::Receive8Error {
                                    step: receive::Step8Error::Data {
                                        sink,
                                        index: 0,
                                        length,
                                    },
                                    error: receive::Error::UnexpectedLength(error),
                                    attempt,
                                    frame: 0,
                                })),
                                None => Ok(Some(Self::Receive8Error {
                                    step: receive::Step8Error::Checksum1 { sink },
                                    error: receive::Error::UnexpectedLength(error),
                                    attempt,
                                    frame: 0,
                                })),
                            },
                        }
                    }
                    receive::Step8::Data { sink, command_xor } => match sink.parse(byte) {
                        Ok(Either::Left(sink)) => Ok(Some(Self::Receive8 {
                            step: receive::Step8::Data { sink, command_xor },
                            checksum: checksum.wrapping_add(byte as u16),
                            attempt,
                            frame: 0,
                        })),
                        Ok(Either::Right(result)) => Ok(Some(Self::Receive8 {
                            step: receive::Step8::Checksum1 {
                                result,
                                command_xor,
                            },
                            checksum: checksum.wrapping_add(byte as u16),
                            attempt,
                            frame: 0,
                        })),
                        Err((error, index, length, sink)) => {
                            if let Some(next_index) = index.checked_add(1)
                                && next_index < length.get()
                            {
                                // We still have more data to receive in the error state.
                                Ok(Some(Self::Receive8Error {
                                    step: receive::Step8Error::Data {
                                        sink,
                                        index: next_index,
                                        length,
                                    },
                                    error: receive::Error::MalformedData(error),
                                    attempt,
                                    frame: 0,
                                }))
                            } else {
                                // The error happened on the last byte being received.
                                Ok(Some(Self::Receive8Error {
                                    step: receive::Step8Error::Checksum1 { sink },
                                    error: receive::Error::MalformedData(error),
                                    attempt,
                                    frame: 0,
                                }))
                            }
                        }
                    },
                    receive::Step8::Checksum1 {
                        result,
                        command_xor,
                    } => Ok(Some(Self::Receive8 {
                        step: receive::Step8::Checksum2 {
                            first_byte: byte,
                            result,
                            command_xor,
                        },
                        checksum,
                        attempt,
                        frame: 0,
                    })),
                    receive::Step8::Checksum2 {
                        result,
                        first_byte,
                        command_xor,
                    } => {
                        let full_checksum = ((first_byte as u16) << 8) | (byte as u16);
                        if full_checksum == checksum {
                            Ok(Some(Self::Receive8 {
                                step: receive::Step8::AcknowledgementSignalDevice {
                                    result,
                                    command_xor,
                                },
                                checksum,
                                attempt,
                                frame: 0,
                            }))
                        } else {
                            Ok(Some(Self::Receive8Error {
                                step: receive::Step8Error::AcknowledgementSignalDevice {
                                    sink: result.revert(),
                                },
                                error: receive::Error::Checksum {
                                    calculated: checksum,
                                    received: full_checksum,
                                },
                                attempt,
                                frame: 0,
                            }))
                        }
                    }
                    receive::Step8::AcknowledgementSignalDevice {
                        result,
                        command_xor,
                    } => match Adapter::try_from(byte) {
                        Ok(received_adapter) => Ok(Some(Self::Receive8 {
                            step: receive::Step8::AcknowledgementSignalCommand {
                                result,
                                adapter: received_adapter,
                                command_xor,
                            },
                            checksum,
                            attempt,
                            frame: 0,
                        })),
                        Err(unknown) => Ok(Some(Self::Receive8Error {
                            step: receive::Step8Error::AcknowledgementSignalCommand {
                                sink: result.revert(),
                            },
                            error: receive::Error::UnsupportedDevice(unknown),
                            attempt,
                            frame: 0,
                        })),
                    },
                    receive::Step8::AcknowledgementSignalCommand {
                        result,
                        adapter: received_adapter,
                        ..
                    } => {
                        // The acknowledgement signal command we receive is expected to be 0x00.
                        match NonZeroU8::new(byte) {
                            None => {
                                // We don't care about what the adapter was set to previously.
                                // We just want to store whatever type it's currently telling
                                // us it is.
                                *adapter = received_adapter;
                                match result.finish() {
                                    sink::Finished::Success => Ok(None),
                                    sink::Finished::TransferLength(new_transfer_length) => {
                                        *transfer_length = new_transfer_length;
                                        unsafe {
                                            // Set SIOCNT so that we can write data to the correct SIODATA.
                                            SIOCNT.write_volatile(
                                                SIOCNT
                                                    .read_volatile()
                                                    .transfer_length(new_transfer_length),
                                            );
                                        }
                                        Ok(None)
                                    }
                                    sink::Finished::CommandError(error) => {
                                        Err(Either::Right(error))
                                    }
                                }
                            }
                            Some(nonzero) => {
                                // We can no longer retry at this point. We simply enter an
                                // error state.
                                Err(Either::Left(Error::Receive(
                                    receive::Error::NonZeroAcknowledgementCommand(nonzero),
                                )))
                            }
                        }
                    }
                }
            }

            // /---------------\
            // | SIO32 Receive |
            // \---------------/
            Self::Receive32 {
                step,
                checksum,
                attempt,
                ..
            } => {
                let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };
                match step {
                    receive::Step32::MagicByte {
                        sink,
                        frame: step_frame,
                    } => match bytes[0] {
                        0xd2 => Ok(Some(Self::Receive32 {
                            step: receive::Step32::MagicByte {
                                sink,
                                frame: step_frame,
                            },
                            checksum,
                            attempt,
                            frame: 0,
                        })),
                        0x99 => match bytes[1] {
                            0x66 => {
                                let command_xor = bytes[2] & 0x80 == 0;
                                match Command::try_from(bytes[2] & 0x7f) {
                                    Ok(command) => match sink.parse(command) {
                                        Ok(sink) => Ok(Some(Self::Receive32 {
                                            step: receive::Step32::HeaderLength {
                                                sink,
                                                command_xor,
                                            },
                                            checksum: checksum
                                                .wrapping_add(bytes[2] as u16)
                                                .wrapping_add(bytes[3] as u16),
                                            attempt,
                                            frame: 0,
                                        })),
                                        Err((error, sink)) => Ok(Some(Self::Receive32Error {
                                            step: receive::Step32Error::HeaderLength { sink },
                                            error: receive::Error::UnsupportedCommand(error),
                                            attempt,
                                            frame: 0,
                                        })),
                                    },
                                    Err(unknown) => Ok(Some(Self::Receive32Error {
                                        step: receive::Step32Error::HeaderLength { sink },
                                        error: receive::Error::UnknownCommand(unknown),
                                        attempt,
                                        frame: 0,
                                    })),
                                }
                            }
                            byte => Ok(Some(Self::Receive32Error {
                                step: receive::Step32Error::HeaderLength { sink },
                                error: receive::Error::MagicValue2(byte),
                                attempt,
                                frame: 0,
                            })),
                        },
                        byte => Ok(Some(Self::Receive32Error {
                            step: receive::Step32Error::HeaderLength { sink },
                            error: receive::Error::MagicValue1(byte),
                            attempt,
                            frame: 0,
                        })),
                    },
                    receive::Step32::HeaderLength { sink, command_xor } => {
                        let full_length = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                        match sink.parse(full_length) {
                            Ok(Either::Left(sink)) => {
                                // Receive the last two bytes as data.
                                match sink.parse(bytes[2]) {
                                    Ok(Either::Left(sink)) => match sink.parse(bytes[3]) {
                                        Ok(Either::Left(sink)) => Ok(Some(Self::Receive32 {
                                            step: receive::Step32::Data { sink, command_xor },
                                            checksum: checksum
                                                .wrapping_add(bytes[0] as u16)
                                                .wrapping_add(bytes[1] as u16)
                                                .wrapping_add(bytes[2] as u16)
                                                .wrapping_add(bytes[3] as u16),
                                            attempt,
                                            frame: 0,
                                        })),
                                        Ok(Either::Right(result)) => Ok(Some(Self::Receive32 {
                                            step: receive::Step32::Checksum {
                                                result,
                                                command_xor,
                                            },
                                            checksum: checksum
                                                .wrapping_add(bytes[0] as u16)
                                                .wrapping_add(bytes[1] as u16)
                                                .wrapping_add(bytes[2] as u16)
                                                .wrapping_add(bytes[3] as u16),
                                            attempt,
                                            frame: 0,
                                        })),
                                        Err((error, index, length, sink)) => {
                                            if let Some(next_index) = index.checked_add(1)
                                                && next_index < length.get()
                                            {
                                                // We still have more data to receive in the error state.
                                                Ok(Some(Self::Receive32Error {
                                                    step: receive::Step32Error::Data {
                                                        sink,
                                                        index: next_index,
                                                        length,
                                                    },
                                                    error: receive::Error::MalformedData(error),
                                                    attempt,
                                                    frame: 0,
                                                }))
                                            } else {
                                                // The error happened on the last byte being received.
                                                Ok(Some(Self::Receive32Error {
                                                    step: receive::Step32Error::Checksum { sink },
                                                    error: receive::Error::MalformedData(error),
                                                    attempt,
                                                    frame: 0,
                                                }))
                                            }
                                        }
                                    },
                                    Ok(Either::Right(result)) => Ok(Some(Self::Receive32 {
                                        step: receive::Step32::Checksum {
                                            result,
                                            command_xor,
                                        },
                                        checksum: checksum
                                            .wrapping_add(bytes[0] as u16)
                                            .wrapping_add(bytes[1] as u16)
                                            .wrapping_add(bytes[2] as u16)
                                            .wrapping_add(bytes[3] as u16),
                                        attempt,
                                        frame: 0,
                                    })),
                                    Err((error, index, length, sink)) => {
                                        if let Some(next_index) = index.checked_add(2)
                                            && next_index < length.get()
                                        {
                                            // We still have more data to receive in the error state.
                                            Ok(Some(Self::Receive32Error {
                                                step: receive::Step32Error::Data {
                                                    sink,
                                                    index: next_index,
                                                    length,
                                                },
                                                error: receive::Error::MalformedData(error),
                                                attempt,
                                                frame: 0,
                                            }))
                                        } else {
                                            // The error happened on the last byte being received.
                                            Ok(Some(Self::Receive32Error {
                                                step: receive::Step32Error::Checksum { sink },
                                                error: receive::Error::MalformedData(error),
                                                attempt,
                                                frame: 0,
                                            }))
                                        }
                                    }
                                }
                            }
                            Ok(Either::Right(result)) => {
                                // No data to receive, so we move right on to the checksum.
                                let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                                if full_checksum == checksum {
                                    Ok(Some(Self::Receive32 {
                                        step: receive::Step32::AcknowledgementSignal {
                                            result,
                                            command_xor,
                                        },
                                        checksum,
                                        attempt,
                                        frame: 0,
                                    }))
                                } else {
                                    Ok(Some(Self::Receive32Error {
                                        step: receive::Step32Error::AcknowledgementSignal {
                                            sink: result.revert(),
                                        },
                                        error: receive::Error::Checksum {
                                            calculated: checksum,
                                            received: full_checksum,
                                        },
                                        attempt,
                                        frame: 0,
                                    }))
                                }
                            }
                            Err((error, sink)) => Ok(Some(Self::Receive32Error {
                                step: receive::Step32Error::AcknowledgementSignal { sink },
                                error: receive::Error::UnexpectedLength(error),
                                attempt,
                                frame: 0,
                            })),
                        }
                    }
                    receive::Step32::Data { sink, command_xor } => {
                        match sink.parse(bytes[0]) {
                            Ok(Either::Left(sink)) => {
                                match sink.parse(bytes[1]) {
                                    Ok(Either::Left(sink)) => {
                                        match sink.parse(bytes[2]) {
                                            Ok(Either::Left(sink)) => {
                                                match sink.parse(bytes[3]) {
                                                    Ok(Either::Left(sink)) => {
                                                        Ok(Some(Self::Receive32 {
                                                            step: receive::Step32::Data {
                                                                sink,
                                                                command_xor,
                                                            },
                                                            checksum: checksum
                                                                .wrapping_add(bytes[0] as u16)
                                                                .wrapping_add(bytes[1] as u16)
                                                                .wrapping_add(bytes[2] as u16)
                                                                .wrapping_add(bytes[3] as u16),
                                                            attempt,
                                                            frame: 0,
                                                        }))
                                                    }
                                                    Ok(Either::Right(result)) => {
                                                        Ok(Some(Self::Receive32 {
                                                            step: receive::Step32::Checksum {
                                                                result,
                                                                command_xor,
                                                            },
                                                            checksum: checksum
                                                                .wrapping_add(bytes[0] as u16)
                                                                .wrapping_add(bytes[1] as u16)
                                                                .wrapping_add(bytes[2] as u16)
                                                                .wrapping_add(bytes[3] as u16),
                                                            attempt,
                                                            frame: 0,
                                                        }))
                                                    }
                                                    Err((error, index, length, sink)) => {
                                                        if let Some(next_index) =
                                                            index.checked_add(1)
                                                            && next_index < length.get()
                                                        {
                                                            // We still have more data to receive in the error state.
                                                            Ok(Some(Self::Receive32Error {
                                                                step: receive::Step32Error::Data {
                                                                    sink,
                                                                    index: next_index,
                                                                    length,
                                                                },
                                                                error:
                                                                    receive::Error::MalformedData(
                                                                        error,
                                                                    ),
                                                                attempt,
                                                                frame: 0,
                                                            }))
                                                        } else {
                                                            // The error happened on the last byte being received.
                                                            Ok(Some(Self::Receive32Error {
                                                                step:
                                                                    receive::Step32Error::Checksum {
                                                                        sink,
                                                                    },
                                                                error:
                                                                    receive::Error::MalformedData(
                                                                        error,
                                                                    ),
                                                                attempt,
                                                                frame: 0,
                                                            }))
                                                        }
                                                    }
                                                }
                                            }
                                            Ok(Either::Right(result)) => {
                                                Ok(Some(Self::Receive32 {
                                                    step: receive::Step32::Checksum {
                                                        result,
                                                        command_xor,
                                                    },
                                                    checksum: checksum
                                                        .wrapping_add(bytes[0] as u16)
                                                        .wrapping_add(bytes[1] as u16)
                                                        .wrapping_add(bytes[2] as u16)
                                                        .wrapping_add(bytes[3] as u16),
                                                    attempt,
                                                    frame: 0,
                                                }))
                                            }
                                            Err((error, index, length, sink)) => {
                                                if let Some(next_index) = index.checked_add(2)
                                                    && next_index < length.get()
                                                {
                                                    // We still have more data to receive in the error state.
                                                    Ok(Some(Self::Receive32Error {
                                                        step: receive::Step32Error::Data {
                                                            sink,
                                                            index: next_index,
                                                            length,
                                                        },
                                                        error: receive::Error::MalformedData(error),
                                                        attempt,
                                                        frame: 0,
                                                    }))
                                                } else {
                                                    // The error happened on the last byte being received.
                                                    Ok(Some(Self::Receive32Error {
                                                        step: receive::Step32Error::Checksum {
                                                            sink,
                                                        },
                                                        error: receive::Error::MalformedData(error),
                                                        attempt,
                                                        frame: 0,
                                                    }))
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
                                            Ok(Some(Self::Receive32 {
                                                step: receive::Step32::AcknowledgementSignal {
                                                    result,
                                                    command_xor,
                                                },
                                                checksum: calculated_checksum,
                                                attempt,
                                                frame: 0,
                                            }))
                                        } else {
                                            Ok(Some(Self::Receive32Error {
                                                step: receive::Step32Error::AcknowledgementSignal {
                                                    sink: result.revert(),
                                                },
                                                error: receive::Error::Checksum {
                                                    calculated: calculated_checksum,
                                                    received: full_checksum,
                                                },
                                                attempt,
                                                frame: 0,
                                            }))
                                        }
                                    }
                                    Err((error, index, length, sink)) => {
                                        if let Some(next_index) = index.checked_add(3)
                                            && next_index < length.get()
                                        {
                                            // We still have more data to receive in the error state.
                                            Ok(Some(Self::Receive32Error {
                                                step: receive::Step32Error::Data {
                                                    sink,
                                                    index: next_index,
                                                    length,
                                                },
                                                error: receive::Error::MalformedData(error),
                                                attempt,
                                                frame: 0,
                                            }))
                                        } else {
                                            // The error happened on the last byte being received.
                                            Ok(Some(Self::Receive32Error {
                                                step: receive::Step32Error::AcknowledgementSignal {
                                                    sink,
                                                },
                                                error: receive::Error::MalformedData(error),
                                                attempt,
                                                frame: 0,
                                            }))
                                        }
                                    }
                                }
                            }
                            Ok(Either::Right(result)) => {
                                // The checksum is contained in the last two bytes.
                                let calculated_checksum = checksum
                                    .wrapping_add(bytes[0] as u16)
                                    .wrapping_add(bytes[1] as u16);
                                let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                                if full_checksum == calculated_checksum {
                                    Ok(Some(Self::Receive32 {
                                        step: receive::Step32::AcknowledgementSignal {
                                            result,
                                            command_xor,
                                        },
                                        checksum: calculated_checksum,
                                        attempt,
                                        frame: 0,
                                    }))
                                } else {
                                    Ok(Some(Self::Receive32Error {
                                        step: receive::Step32Error::AcknowledgementSignal {
                                            sink: result.revert(),
                                        },
                                        error: receive::Error::Checksum {
                                            calculated: calculated_checksum,
                                            received: full_checksum,
                                        },
                                        attempt,
                                        frame: 0,
                                    }))
                                }
                            }
                            Err((error, index, length, sink)) => {
                                if let Some(next_index) = index.checked_add(4)
                                    && next_index < length.get()
                                {
                                    // We still have more data to receive in the error state.
                                    Ok(Some(Self::Receive32Error {
                                        step: receive::Step32Error::Data {
                                            sink,
                                            index: next_index,
                                            length,
                                        },
                                        error: receive::Error::MalformedData(error),
                                        attempt,
                                        frame: 0,
                                    }))
                                } else {
                                    // The error happened on the last byte being received.
                                    Ok(Some(Self::Receive32Error {
                                        step: receive::Step32Error::AcknowledgementSignal { sink },
                                        error: receive::Error::MalformedData(error),
                                        attempt,
                                        frame: 0,
                                    }))
                                }
                            }
                        }
                    }
                    receive::Step32::Checksum {
                        result,
                        command_xor,
                    } => {
                        // The checksum is contained in the last two bytes.
                        let calculated_checksum = checksum
                            .wrapping_add(bytes[0] as u16)
                            .wrapping_add(bytes[1] as u16);
                        let full_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
                        if full_checksum == calculated_checksum {
                            Ok(Some(Self::Receive32 {
                                step: receive::Step32::AcknowledgementSignal {
                                    result,
                                    command_xor,
                                },
                                checksum: calculated_checksum,
                                attempt,
                                frame: 0,
                            }))
                        } else {
                            Ok(Some(Self::Receive32Error {
                                step: receive::Step32Error::AcknowledgementSignal {
                                    sink: result.revert(),
                                },
                                error: receive::Error::Checksum {
                                    calculated: calculated_checksum,
                                    received: full_checksum,
                                },
                                attempt,
                                frame: 0,
                            }))
                        }
                    }
                    receive::Step32::AcknowledgementSignal { result, .. } => {
                        match Adapter::try_from(bytes[0]) {
                            Ok(received_adapter) => match NonZeroU8::new(bytes[1]) {
                                None => {
                                    // We don't care about what the adapter was set to previously.
                                    // We just want to store whatever type it's currently telling
                                    // us it is.
                                    *adapter = received_adapter;
                                    match result.finish() {
                                        sink::Finished::Success => Ok(None),
                                        sink::Finished::TransferLength(new_transfer_length) => {
                                            *transfer_length = new_transfer_length;
                                            unsafe {
                                                // Set SIOCNT so that we can write data to the correct SIODATA.
                                                SIOCNT.write_volatile(
                                                    SIOCNT
                                                        .read_volatile()
                                                        .transfer_length(new_transfer_length),
                                                );
                                            }
                                            Ok(None)
                                        }
                                        sink::Finished::CommandError(error) => {
                                            Err(Either::Right(error))
                                        }
                                    }
                                }
                                Some(nonzero) => {
                                    // We can no longer retry at this point. We simply enter an
                                    // error state.
                                    Err(Either::Left(Error::Receive(
                                        receive::Error::NonZeroAcknowledgementCommand(nonzero),
                                    )))
                                }
                            },
                            Err(unknown) => {
                                // We can no longer retry at this point. We simply enter an
                                // error state.
                                Err(Either::Left(Error::Receive(
                                    receive::Error::UnsupportedDevice(unknown),
                                )))
                            }
                        }
                    }
                }
            }

            // /--------------------\
            // | SIO8 Receive Error |
            // \--------------------/
            Self::Receive8Error {
                step,
                error,
                attempt,
                ..
            } => {
                let byte = unsafe { SIODATA8.read_volatile() };
                log::debug!("received byte {byte:#04x}");
                match step {
                    receive::Step8Error::MagicByte2 { sink } => Ok(Some(Self::Receive8Error {
                        step: receive::Step8Error::HeaderCommand { sink },
                        error,
                        attempt,
                        frame: 0,
                    })),
                    receive::Step8Error::HeaderCommand { sink } => Ok(Some(Self::Receive8Error {
                        step: receive::Step8Error::HeaderEmptyByte { sink },
                        error,
                        attempt,
                        frame: 0,
                    })),
                    receive::Step8Error::HeaderEmptyByte { sink } => {
                        Ok(Some(Self::Receive8Error {
                            step: receive::Step8Error::HeaderLength1 { sink },
                            error,
                            attempt,
                            frame: 0,
                        }))
                    }
                    receive::Step8Error::HeaderLength1 { sink } => Ok(Some(Self::Receive8Error {
                        step: receive::Step8Error::HeaderLength2 {
                            sink,
                            first_byte: byte,
                        },
                        error,
                        attempt,
                        frame: 0,
                    })),
                    receive::Step8Error::HeaderLength2 { sink, first_byte } => {
                        let full_length = ((first_byte as u16) << 8) | (byte as u16);
                        match NonZeroU16::new(full_length) {
                            Some(length) => Ok(Some(Self::Receive8Error {
                                step: receive::Step8Error::Data {
                                    sink,
                                    index: 0,
                                    length,
                                },
                                error,
                                attempt,
                                frame: 0,
                            })),
                            None => Ok(Some(Self::Receive8Error {
                                step: receive::Step8Error::Checksum1 { sink },
                                error,
                                attempt,
                                frame: 0,
                            })),
                        }
                    }
                    receive::Step8Error::Data {
                        sink,
                        index,
                        length,
                    } => {
                        let next_index = index + 1;
                        if next_index < length.get() {
                            Ok(Some(Self::Receive8Error {
                                step: receive::Step8Error::Data {
                                    sink,
                                    index: next_index,
                                    length,
                                },
                                error,
                                attempt,
                                frame: 0,
                            }))
                        } else {
                            Ok(Some(Self::Receive8Error {
                                step: receive::Step8Error::Checksum1 { sink },
                                error,
                                attempt,
                                frame: 0,
                            }))
                        }
                    }
                    receive::Step8Error::Checksum1 { sink } => Ok(Some(Self::Receive8Error {
                        step: receive::Step8Error::Checksum2 { sink },
                        error,
                        attempt,
                        frame: 0,
                    })),
                    receive::Step8Error::Checksum2 { sink } => Ok(Some(Self::Receive8Error {
                        step: receive::Step8Error::AcknowledgementSignalDevice { sink },
                        error,
                        attempt,
                        frame: 0,
                    })),
                    receive::Step8Error::AcknowledgementSignalDevice { sink } => {
                        Ok(Some(Self::Receive8Error {
                            step: receive::Step8Error::AcknowledgementSignalCommand { sink },
                            error,
                            attempt,
                            frame: 0,
                        }))
                    }
                    receive::Step8Error::AcknowledgementSignalCommand { sink } => {
                        let new_attempt = attempt + 1;
                        if new_attempt < MAX_RETRIES {
                            // Retry.
                            Ok(Some(Self::Receive8 {
                                step: receive::Step8::MagicByte1 { sink, frame: 0 },
                                checksum: 0,
                                attempt: new_attempt,
                                frame: 0,
                            }))
                        } else {
                            // Too many retries. Stop trying and set error state.
                            Err(Either::Left(Error::Receive(error)))
                        }
                    }
                }
            }

            // /---------------------\
            // | SIO32 Receive Error |
            // \---------------------/
            Self::Receive32Error {
                step,
                error,
                attempt,
                ..
            } => {
                let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };
                match step {
                    receive::Step32Error::HeaderLength { sink } => {
                        let full_length = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
                        match NonZeroU16::new(full_length) {
                            Some(length) => {
                                if 2 < length.get() {
                                    Ok(Some(Self::Receive32Error {
                                        step: receive::Step32Error::Checksum { sink },
                                        error,
                                        attempt,
                                        frame: 0,
                                    }))
                                } else {
                                    Ok(Some(Self::Receive32Error {
                                        step: receive::Step32Error::Data {
                                            sink,
                                            index: 2,
                                            length,
                                        },
                                        error,
                                        attempt,
                                        frame: 0,
                                    }))
                                }
                            }
                            None => Ok(Some(Self::Receive32Error {
                                step: receive::Step32Error::AcknowledgementSignal { sink },
                                error,
                                attempt,
                                frame: 0,
                            })),
                        }
                    }
                    receive::Step32Error::Data {
                        sink,
                        index,
                        length,
                    } => {
                        if index + 2 >= length.get() {
                            // Checksum is included in last two bytes.
                            Ok(Some(Self::Receive32Error {
                                step: receive::Step32Error::AcknowledgementSignal { sink },
                                error,
                                attempt,
                                frame: 0,
                            }))
                        } else if index + 4 >= length.get() {
                            // These are the last data bytes.
                            Ok(Some(Self::Receive32Error {
                                step: receive::Step32Error::Checksum { sink },
                                error,
                                attempt,
                                frame: 0,
                            }))
                        } else {
                            // There is more data.
                            Ok(Some(Self::Receive32Error {
                                step: receive::Step32Error::Data {
                                    sink,
                                    index: index + 4,
                                    length,
                                },
                                error,
                                attempt,
                                frame: 0,
                            }))
                        }
                    }
                    receive::Step32Error::Checksum { sink } => Ok(Some(Self::Receive32Error {
                        step: receive::Step32Error::AcknowledgementSignal { sink },
                        error,
                        attempt,
                        frame: 0,
                    })),
                    receive::Step32Error::AcknowledgementSignal { sink } => {
                        let new_attempt = attempt + 1;
                        if new_attempt < MAX_RETRIES {
                            // Retry.
                            Ok(Some(Self::Receive32 {
                                step: receive::Step32::MagicByte { sink, frame: 0 },
                                checksum: 0,
                                attempt: new_attempt,
                                frame: 0,
                            }))
                        } else {
                            // Too many retries. Stop trying and set error state.
                            Err(Either::Left(Error::Receive(error)))
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Packet;
    use crate::{
        engine::{Adapter, Command, HANDSHAKE, Source, command},
        mmio::serial::{self, Mode, RCNT, SIOCNT, SIODATA8, SIODATA32, TransferLength},
    };
    use claims::{assert_err_eq, assert_none, assert_ok, assert_some};
    use either::Either;
    use gba_test::test;

    macro_rules! assert_sio_8 {
        ($packet:ident, $adapter:ident, $transfer_length:ident, $send:expr, $receive:expr $(,)?) => {
            #[allow(unused_assignments)] // It's okay if we don't use the packet after this.
            {
                $packet.push();
                assert_eq!(unsafe { SIODATA8.read_volatile() }, $send);
                unsafe { SIODATA8.write_volatile($receive) };
                $packet = assert_some!(assert_ok!(
                    $packet.pull(&mut $adapter, &mut $transfer_length)
                ));
            }
        };
    }

    macro_rules! assert_sio_8_final {
        ($packet:ident, $adapter:ident, $transfer_length:ident, $send:expr, $receive:expr $(,)?) => {
            $packet.push();
            assert_eq!(unsafe { SIODATA8.read_volatile() }, $send);
            unsafe { SIODATA8.write_volatile($receive) };
            assert_none!(assert_ok!(
                $packet.pull(&mut $adapter, &mut $transfer_length)
            ));
        };
    }

    macro_rules! assert_sio_8_final_error {
        ($packet:ident, $adapter:ident, $transfer_length:ident, $send:expr, $receive:expr, $error:expr $(,)?) => {
            $packet.push();
            assert_eq!(unsafe { SIODATA8.read_volatile() }, $send);
            unsafe { SIODATA8.write_volatile($receive) };
            assert_err_eq!($packet.pull(&mut $adapter, &mut $transfer_length), $error,);
        };
    }

    macro_rules! assert_sio_32 {
        ($packet:ident, $adapter:ident, $transfer_length:ident, $send:expr, $receive:expr $(,)?) => {
            #[allow(unused_assignments)] // It's okay if we don't use the packet after this.
            {
                $packet.push();
                assert_eq!(unsafe { SIODATA32.read_volatile() }, $send);
                unsafe { SIODATA32.write_volatile($receive) };
                $packet = assert_some!(assert_ok!(
                    $packet.pull(&mut $adapter, &mut $transfer_length)
                ));
            }
        };
    }

    macro_rules! assert_sio_32_final {
        ($packet:ident, $adapter:ident, $transfer_length:ident, $send:expr, $receive:expr $(,)?) => {
            $packet.push();
            assert_eq!(unsafe { SIODATA32.read_volatile() }, $send);
            unsafe { SIODATA32.write_volatile($receive) };
            assert_none!(assert_ok!(
                $packet.pull(&mut $adapter, &mut $transfer_length)
            ));
        };
    }

    macro_rules! assert_sio_32_final_error {
        ($packet:ident, $adapter:ident, $transfer_length:ident, $send:expr, $receive:expr, $error:expr $(,)?) => {
            $packet.push();
            assert_eq!(unsafe { SIODATA32.read_volatile() }, $send);
            unsafe { SIODATA32.write_volatile($receive) };
            assert_err_eq!($packet.pull(&mut $adapter, &mut $transfer_length), $error,);
        };
    }

    #[test]
    fn begin_session_send8() {
        // Enter Normal SIO8 mode so that SIODATA can be used.
        let mut transfer_length = TransferLength::_8Bit;
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(transfer_length));
        }

        let mut packet = Packet::new(transfer_length, Source::BeginSession);
        let mut adapter = Adapter::Blue;

        // /------\
        // | Send |
        // \------/

        // Magic values.
        assert_sio_8!(packet, adapter, transfer_length, 0x99, 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, 0x66, 0xd2);

        // Header.
        // Command.
        assert_sio_8!(
            packet,
            adapter,
            transfer_length,
            Command::BeginSession as u8,
            0xd2
        );
        assert_sio_8!(packet, adapter, transfer_length, 0x00, 0xd2);
        // Length.
        assert_sio_8!(packet, adapter, transfer_length, 0x00, 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, 0x08, 0xd2);

        // Data.
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[0], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[1], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[2], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[3], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[4], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[5], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[6], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[7], 0xd2);

        // Checksum.
        assert_sio_8!(packet, adapter, transfer_length, 0x02, 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, 0x77, 0xd2);

        // Acknowledgement Signal.
        assert_sio_8!(packet, adapter, transfer_length, 0x81, Adapter::Red as u8);
        assert_sio_8!(
            packet,
            adapter,
            transfer_length,
            0x00,
            Command::BeginSession as u8 ^ 0x80
        );

        // /---------\
        // | Receive |
        // \---------/

        // Magic values.
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x99);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x66);

        // Header.
        // Command.
        assert_sio_8!(
            packet,
            adapter,
            transfer_length,
            0x4b,
            Command::BeginSession as u8
        );
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x00);
        // Length.
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x00);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x08);

        // Data.
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, HANDSHAKE[0]);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, HANDSHAKE[1]);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, HANDSHAKE[2]);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, HANDSHAKE[3]);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, HANDSHAKE[4]);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, HANDSHAKE[5]);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, HANDSHAKE[6]);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, HANDSHAKE[7]);

        // Checksum.
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x02);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x77);

        // Acknowledgement Signal.
        assert_sio_8!(packet, adapter, transfer_length, 0x81, Adapter::Blue as u8);
        assert_sio_8_final!(
            packet,
            adapter,
            transfer_length,
            Command::BeginSession as u8 ^ 0x80,
            0x00
        );
    }

    #[test]
    fn begin_session_send8_command_error() {
        // Enter Normal SIO8 mode so that SIODATA can be used.
        let mut transfer_length = TransferLength::_8Bit;
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(transfer_length));
        }

        let mut packet = Packet::new(transfer_length, Source::BeginSession);
        let mut adapter = Adapter::Blue;

        // /------\
        // | Send |
        // \------/

        // Magic values.
        assert_sio_8!(packet, adapter, transfer_length, 0x99, 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, 0x66, 0xd2);

        // Header.
        // Command.
        assert_sio_8!(
            packet,
            adapter,
            transfer_length,
            Command::BeginSession as u8,
            0xd2
        );
        assert_sio_8!(packet, adapter, transfer_length, 0x00, 0xd2);
        // Length.
        assert_sio_8!(packet, adapter, transfer_length, 0x00, 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, 0x08, 0xd2);

        // Data.
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[0], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[1], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[2], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[3], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[4], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[5], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[6], 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, HANDSHAKE[7], 0xd2);

        // Checksum.
        assert_sio_8!(packet, adapter, transfer_length, 0x02, 0xd2);
        assert_sio_8!(packet, adapter, transfer_length, 0x77, 0xd2);

        // Acknowledgement Signal.
        assert_sio_8!(packet, adapter, transfer_length, 0x81, Adapter::Red as u8);
        assert_sio_8!(
            packet,
            adapter,
            transfer_length,
            0x00,
            Command::BeginSession as u8 ^ 0x80
        );

        // /---------\
        // | Receive |
        // \---------/

        // Magic values.
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x99);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x66);

        // Header.
        // Command.
        assert_sio_8!(
            packet,
            adapter,
            transfer_length,
            0x4b,
            Command::CommandError as u8
        );
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x00);
        // Length.
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x00);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x02);

        // Data.
        assert_sio_8!(
            packet,
            adapter,
            transfer_length,
            0x4b,
            Command::BeginSession as u8
        );
        assert_sio_8!(
            packet,
            adapter,
            transfer_length,
            0x4b,
            command::error::begin_session::Error::AlreadyActive as u8
        );

        // Checksum.
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x00);
        assert_sio_8!(packet, adapter, transfer_length, 0x4b, 0x81);

        // Acknowledgement Signal.
        assert_sio_8!(packet, adapter, transfer_length, 0x81, Adapter::Blue as u8);
        assert_sio_8_final_error!(
            packet,
            adapter,
            transfer_length,
            Command::CommandError as u8 ^ 0x80,
            0x00,
            Either::Right(command::Error::BeginSession(
                command::error::begin_session::Error::AlreadyActive
            )),
        );
    }

    #[test]
    fn begin_session_send32() {
        // Enter Normal SIO32 mode so that SIODATA can be used.
        let mut transfer_length = TransferLength::_32Bit;
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(transfer_length));
        }

        let mut packet = Packet::new(transfer_length, Source::BeginSession);
        let mut adapter = Adapter::Blue;

        // /------\
        // | Send |
        // \------/

        // Magic values + command.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([0x99, 0x66, Command::BeginSession as u8, 0x00]),
            0xd2_d2_d2_d2,
        );

        // Length + Data.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([0x00, 0x08, HANDSHAKE[0], HANDSHAKE[1]]),
            0xd2_d2_d2_d2,
        );
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([HANDSHAKE[2], HANDSHAKE[3], HANDSHAKE[4], HANDSHAKE[5]]),
            0xd2_d2_d2_d2,
        );
        // Data + Checksum
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([HANDSHAKE[6], HANDSHAKE[7], 0x02, 0x77]),
            0xd2_d2_d2_d2,
        );

        // Acknowledgement Signal.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([0x81, 0x00, 0x00, 0x00]),
            u32::from_be_bytes([
                Adapter::Blue as u8,
                Command::BeginSession as u8 ^ 0x80,
                0x00,
                0x00,
            ]),
        );

        // /---------\
        // | Receive |
        // \---------/

        // Magic values + command.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([0x99, 0x66, Command::BeginSession as u8, 0x00])
        );

        // Length + Data.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([0x00, 0x08, HANDSHAKE[0], HANDSHAKE[1]]),
        );
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([HANDSHAKE[2], HANDSHAKE[3], HANDSHAKE[4], HANDSHAKE[5]]),
        );
        // Data + Checksum
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([HANDSHAKE[6], HANDSHAKE[7], 0x02, 0x77]),
        );

        // Acknowledgement Signal.
        assert_sio_32_final!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([0x81, Command::BeginSession as u8 ^ 0x80, 0x00, 0x00]),
            u32::from_be_bytes([Adapter::Blue as u8, 0x00, 0x00, 0x00]),
        );
    }

    #[test]
    fn begin_session_send32_command_error() {
        // Enter Normal SIO32 mode so that SIODATA can be used.
        let mut transfer_length = TransferLength::_32Bit;
        unsafe {
            RCNT.write_volatile(Mode::NORMAL);
            SIOCNT.write_volatile(serial::Control::new().transfer_length(transfer_length));
        }

        let mut packet = Packet::new(transfer_length, Source::BeginSession);
        let mut adapter = Adapter::Blue;

        // /------\
        // | Send |
        // \------/

        // Magic values + command.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([0x99, 0x66, Command::BeginSession as u8, 0x00]),
            0xd2_d2_d2_d2,
        );

        // Length + Data.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([0x00, 0x08, HANDSHAKE[0], HANDSHAKE[1]]),
            0xd2_d2_d2_d2,
        );
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([HANDSHAKE[2], HANDSHAKE[3], HANDSHAKE[4], HANDSHAKE[5]]),
            0xd2_d2_d2_d2,
        );
        // Data + Checksum
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([HANDSHAKE[6], HANDSHAKE[7], 0x02, 0x77]),
            0xd2_d2_d2_d2,
        );

        // Acknowledgement Signal.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([0x81, 0x00, 0x00, 0x00]),
            u32::from_be_bytes([
                Adapter::Blue as u8,
                Command::BeginSession as u8 ^ 0x80,
                0x00,
                0x00,
            ]),
        );

        // /---------\
        // | Receive |
        // \---------/

        // Magic values + command.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([0x99, 0x66, Command::CommandError as u8, 0x00])
        );

        // Length + Data.
        assert_sio_32!(
            packet,
            adapter,
            transfer_length,
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
            packet,
            adapter,
            transfer_length,
            0x4b_4b_4b_4b,
            u32::from_be_bytes([0x00, 0x00, 0x00, 0x81]),
        );

        // Acknowledgement Signal.
        assert_sio_32_final_error!(
            packet,
            adapter,
            transfer_length,
            u32::from_be_bytes([0x81, Command::CommandError as u8 ^ 0x80, 0x00, 0x00]),
            u32::from_be_bytes([Adapter::Blue as u8, 0x00, 0x00, 0x00]),
            Either::Right(command::Error::BeginSession(
                command::error::begin_session::Error::AlreadyActive
            )),
        );
    }
}
