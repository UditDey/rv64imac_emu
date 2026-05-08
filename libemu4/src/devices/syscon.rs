use std::ops::Range;

use crate::bus::BusTxSize;

pub struct Syscon {
    pub power_off: bool,
}

impl Syscon {
    const BASE_ADDR: u64 = 0x1110_0000;
    const SIZE: u64 = 0x1000; // 4 KiB window

    pub const ADDR_RANGE: Range<u64> = Self::BASE_ADDR..(Self::BASE_ADDR + Self::SIZE);

    pub fn new() -> Self {
        Self { power_off: false }
    }

    pub fn load(&self, _addr: u64, _size: BusTxSize) -> u64 {
        0
    }

    pub fn store(&mut self, addr: u64, val: u64, size: BusTxSize) {
        assert!(size == BusTxSize::Bits32);
        if addr != Self::BASE_ADDR {
            return;
        }

        match val as u32 {
            0x5555 => {
                println!("[emu4] Power-off via syscon");
                self.power_off = true;
            }
            0x7777 => panic!("[emu4] Reboot via syscon"),
            _ => {} // ignore anything else
        }

        //crossterm::terminal::disable_raw_mode().unwrap();
    }
}
