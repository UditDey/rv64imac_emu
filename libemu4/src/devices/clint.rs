use std::ops::Range;

use crate::{bus::BusTxSize, csr::Csr};

#[derive(Debug)]
pub struct Clint {
    msip: u64,
    mtimecmp: u64,
    mtime: u64,
}

impl Clint {
    const BASE_ADDR: u64 = 0x2000000;
    const SIZE: u64 = 0x10000;

    pub const ADDR_RANGE: Range<u64> = Self::BASE_ADDR..(Self::BASE_ADDR + Self::SIZE);

    const MSIP_ADDR: u64 = Self::BASE_ADDR;
    const MTIMECMP_ADDR: u64 = Self::BASE_ADDR + 0x4000;
    const MTIME_ADDR: u64 = Self::BASE_ADDR + 0xBFF8;

    pub fn new() -> Self {
        Self {
            msip: 0,
            mtimecmp: u64::MAX,
            mtime: 0,
        }
    }

    pub fn load(&self, addr: u64, size: BusTxSize) -> u64 {
        assert!(size == BusTxSize::Bits64);

        match addr {
            Self::MSIP_ADDR => self.msip,
            Self::MTIMECMP_ADDR => self.mtimecmp,
            Self::MTIME_ADDR => self.mtime,
            _ => panic!(),
        }
    }

    pub fn store(&mut self, addr: u64, val: u64, size: BusTxSize) {
        let val = match size {
            BusTxSize::Bits8 => val & 0xFF,
            BusTxSize::Bits16 => val & 0xFFFF,
            BusTxSize::Bits32 => val & 0xFFFFFFFF,
            BusTxSize::Bits64 => val,
        };

        match addr {
            Self::MSIP_ADDR => self.msip = val,
            Self::MTIMECMP_ADDR => self.mtimecmp = val,
            Self::MTIME_ADDR => self.mtime = val,
            _ => panic!(),
        };
    }

    pub fn tick(&mut self, csr: &mut Csr, delta: u64) {
        self.mtime = delta;
        csr.set_time(self.mtime);
    }

    pub fn tick_cosim(&mut self, delta: u64) -> bool {
        self.mtime = delta;
        self.mtime >= self.mtimecmp
    }

    pub fn poll_interrupts(&self, csr: &mut Csr) {
        csr.update_mip(|mip| {
            mip.set_msip(self.msip & 1 != 0);
            mip.set_mtip(self.mtime >= self.mtimecmp);
            if self.mtime >= self.mtimecmp {
                //print!("emu4: mtip high");
            }
        });
    }
}
