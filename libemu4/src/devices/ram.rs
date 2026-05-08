use std::ops::Range;

use crate::bus::BusTxSize;

pub struct Ram(pub Vec<u8>);

impl Ram {
    pub const BASE_ADDR: u64 = 0x8000_0000;
    pub const SIZE: u64 = 512 * 0x100000; // 512 MiB

    pub const ADDR_RANGE: Range<u64> = Self::BASE_ADDR..(Self::BASE_ADDR + Self::SIZE);

    pub fn new() -> Self {
        Self(vec![0; Self::SIZE as usize])
    }

    pub fn load(&self, addr: u64, size: BusTxSize) -> u64 {
        let addr = (addr - Self::BASE_ADDR) as usize;

        match size {
            BusTxSize::Bits8 => self.0[addr] as u64,
            BusTxSize::Bits16 => u16::from_le_bytes([self.0[addr], self.0[addr + 1]]) as u64,
            BusTxSize::Bits32 => u32::from_le_bytes([
                self.0[addr],
                self.0[addr + 1],
                self.0[addr + 2],
                self.0[addr + 3],
            ]) as u64,
            BusTxSize::Bits64 => u64::from_le_bytes([
                self.0[addr],
                self.0[addr + 1],
                self.0[addr + 2],
                self.0[addr + 3],
                self.0[addr + 4],
                self.0[addr + 5],
                self.0[addr + 6],
                self.0[addr + 7],
            ]),
        }
    }

    pub fn store(&mut self, addr: u64, val: u64, size: BusTxSize) {
        let addr = (addr - Self::BASE_ADDR) as usize;
        let val = val.to_le_bytes();

        match size {
            BusTxSize::Bits8 => self.0[addr] = val[0],
            BusTxSize::Bits16 => {
                self.0[addr] = val[0];
                self.0[addr + 1] = val[1];
            }
            BusTxSize::Bits32 => {
                self.0[addr] = val[0];
                self.0[addr + 1] = val[1];
                self.0[addr + 2] = val[2];
                self.0[addr + 3] = val[3];
            }
            BusTxSize::Bits64 => {
                self.0[addr] = val[0];
                self.0[addr + 1] = val[1];
                self.0[addr + 2] = val[2];
                self.0[addr + 3] = val[3];
                self.0[addr + 4] = val[4];
                self.0[addr + 5] = val[5];
                self.0[addr + 6] = val[6];
                self.0[addr + 7] = val[7];
            }
        }
    }
}
