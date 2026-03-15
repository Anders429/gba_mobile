pub(in crate::driver) mod idle;
pub(in crate::driver) mod packet;
pub(in crate::driver) mod wait_for_idle;

pub(super) use idle::Idle;
pub(super) use packet::Packet;
pub(super) use wait_for_idle::WaitForIdle;

use crate::{
    Timer,
    driver::timers,
    mmio::{
        serial::{self, SIOCNT, TransferLength},
        timer::{self, TM0CNT, TM0VAL, TM1CNT, TM1VAL, TM2CNT, TM2VAL, TM3CNT, TM3VAL},
    },
};

fn schedule_serial(transfer_length: TransferLength) {
    unsafe {
        SIOCNT.write_volatile(
            serial::Control::new()
                .master(true)
                .start(true)
                .interrupts(true)
                .transfer_length(transfer_length),
        )
    }
}

fn schedule_timer(timer: Timer, transfer_length: TransferLength) {
    let value = match transfer_length {
        TransferLength::_8Bit => timers::MICROSECONDS_200,
        TransferLength::_32Bit => timers::MICROSECONDS_400,
    };
    let control = timer::Control::new()
        .frequency(timer::Frequency::_1024)
        .interrupts(true)
        .start(true);
    unsafe {
        match timer {
            Timer::_0 => {
                TM0VAL.write_volatile(value);
                TM0CNT.write_volatile(control);
            }
            Timer::_1 => {
                TM1VAL.write_volatile(value);
                TM1CNT.write_volatile(control);
            }
            Timer::_2 => {
                TM2VAL.write_volatile(value);
                TM2CNT.write_volatile(control);
            }
            Timer::_3 => {
                TM3VAL.write_volatile(value);
                TM3CNT.write_volatile(control);
            }
        }
    }
}
