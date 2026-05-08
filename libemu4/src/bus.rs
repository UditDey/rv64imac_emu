use crate::{
    csr::Csr,
    devices::{clint::Clint, plic::Plic, ram::Ram, syscon::Syscon, uart::Uart},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BusTxSize {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
}

pub struct Bus {
    pub atomic_reservation: Option<u64>,
    pub ram: Ram,
    pub syscon: Syscon,
    pub clint: Clint,
    pub plic: Plic,
    pub uart: Uart,
    pub last_load: Option<(u64, BusTxSize)>,
    pub pending_store: Option<(u64, u64, BusTxSize)>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            atomic_reservation: None,
            ram: Ram::new(),
            syscon: Syscon::new(),
            clint: Clint::new(),
            plic: Plic::new(),
            uart: Uart::new(),
            last_load: None,
            pending_store: None,
        }
    }

    pub fn fetch(&mut self, addr: u64) -> Option<u32> {
        let addr = addr & 0xFFFF_FFFF;

        if Ram::ADDR_RANGE.contains(&addr) {
            Some(self.ram.load(addr, BusTxSize::Bits32) as u32)
        } else {
            None
        }
    }

    pub fn load(&mut self, addr: u64, tx_size: BusTxSize) -> Option<u64> {
        let addr = addr & 0xFFFF_FFFF;

        if Ram::ADDR_RANGE.contains(&addr) {
            let val = self.ram.load(addr, tx_size);
            self.last_load = Some((addr, tx_size));
            Some(val)
        } else if Syscon::ADDR_RANGE.contains(&addr) {
            let val = self.syscon.load(addr, tx_size);
            self.last_load = Some((addr, tx_size));
            Some(val)
        } else if Clint::ADDR_RANGE.contains(&addr) {
            let val = self.clint.load(addr, tx_size);
            self.last_load = Some((addr, tx_size));
            Some(val)
        } else if Plic::ADDR_RANGE.contains(&addr) {
            let val = self.plic.load(addr, tx_size);
            self.last_load = Some((addr, tx_size));
            Some(val)
        } else if Uart::ADDR_RANGE.contains(&addr) {
            let val = self.uart.load(addr, tx_size, &mut self.plic);
            self.last_load = Some((addr, tx_size));
            Some(val)
        } else {
            None
        }
    }

    pub fn load_internal(&mut self, addr: u64, tx_size: BusTxSize) -> Option<u64> {
        if Ram::ADDR_RANGE.contains(&addr) {
            let val = self.ram.load(addr, tx_size);
            Some(val)
        } else if Syscon::ADDR_RANGE.contains(&addr) {
            let val = self.syscon.load(addr, tx_size);
            Some(val)
        } else if Clint::ADDR_RANGE.contains(&addr) {
            let val = self.clint.load(addr, tx_size);
            Some(val)
        } else if Plic::ADDR_RANGE.contains(&addr) {
            let val = self.plic.load(addr, tx_size);
            Some(val)
        } else if Uart::ADDR_RANGE.contains(&addr) {
            let val = self.uart.load(addr, tx_size, &mut self.plic);
            Some(val)
        } else {
            None
        }
    }

    pub fn store(&mut self, addr: u64, val: u64, tx_size: BusTxSize) -> Option<()> {
        let val = match tx_size {
            BusTxSize::Bits8 => val as u8 as u64,
            BusTxSize::Bits16 => val as u16 as u64,
            BusTxSize::Bits32 => val as u32 as u64,
            BusTxSize::Bits64 => val,
        };

        let res_addr = match tx_size {
            BusTxSize::Bits64 => addr,
            BusTxSize::Bits32 => addr & !0x3,
            BusTxSize::Bits16 => addr & !0xF,
            BusTxSize::Bits8 => addr & !0xFF,
        };

        match self.atomic_reservation {
            Some(x) if x == res_addr => {
                //println!("bus store(): Clearing reservation 0x{res_addr:X}");
                //self.atomic_reservation.take();
            }
            _ => (),
        }

        assert!(self.pending_store.is_none());
        self.pending_store = Some((addr, val, tx_size));
        Some(())
    }

    pub fn store_internal(&mut self, addr: u64, val: u64, tx_size: BusTxSize) -> Option<()> {
        let val = match tx_size {
            BusTxSize::Bits8 => val as u8 as u64,
            BusTxSize::Bits16 => val as u16 as u64,
            BusTxSize::Bits32 => val as u32 as u64,
            BusTxSize::Bits64 => val,
        };

        let res_addr = match tx_size {
            BusTxSize::Bits64 => addr,
            BusTxSize::Bits32 => addr & !0x3,
            BusTxSize::Bits16 => addr & !0xF,
            BusTxSize::Bits8 => addr & !0xFF,
        };

        match self.atomic_reservation {
            Some(x) if x == res_addr => {
                //println!("bus store_internal(): Clearing reservation 0x{res_addr:X}");
                //self.atomic_reservation.take();
            }
            _ => (),
        }

        if Ram::ADDR_RANGE.contains(&addr) {
            self.ram.store(addr, val, tx_size);
        } else if Syscon::ADDR_RANGE.contains(&addr) {
            self.syscon.store(addr, val, tx_size);
        } else if Clint::ADDR_RANGE.contains(&addr) {
            self.clint.store(addr, val, tx_size);
        } else if Plic::ADDR_RANGE.contains(&addr) {
            self.plic.store(addr, val, tx_size);
        } else if Uart::ADDR_RANGE.contains(&addr) {
            self.uart.store(addr, val, tx_size, &mut self.plic);
        } else {
            return None;
        }

        Some(())
    }

    pub fn commit_store(&mut self) -> Option<()> {
        if let Some((addr, val, tx_size)) = self.pending_store {
            if Ram::ADDR_RANGE.contains(&addr) {
                self.ram.store(addr, val, tx_size);
            } else if Syscon::ADDR_RANGE.contains(&addr) {
                self.syscon.store(addr, val, tx_size);
            } else if Clint::ADDR_RANGE.contains(&addr) {
                self.clint.store(addr, val, tx_size);
            } else if Plic::ADDR_RANGE.contains(&addr) {
                self.plic.store(addr, val, tx_size);
            } else if Uart::ADDR_RANGE.contains(&addr) {
                self.uart.store(addr, val, tx_size, &mut self.plic);
            } else {
                return None;
            }
        }

        Some(())
    }

    pub fn tick(&mut self, csr: &mut Csr, delta: u64) {
        self.last_load = None;
        self.pending_store = None;
        self.clint.tick(csr, delta);
        self.uart.tick(&mut self.plic);
    }

    /// Checks devices for interrupts and sets the corresponding MIP flags
    pub fn poll_interrupts(&self, csr: &mut Csr) {
        self.clint.poll_interrupts(csr);
        self.plic.poll_interrupts(csr);
    }
}
