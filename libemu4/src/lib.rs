pub mod bus;
pub mod csr;
pub mod decode;
pub mod devices;
pub mod exec;
pub mod mmu;
pub mod payload;
pub mod trap;

use num_enum::{IntoPrimitive, TryFromPrimitive};

use bus::Bus;
use csr::Csr;
use mmu::Mmu;
use trap::Exception;

#[derive(TryFromPrimitive, IntoPrimitive, PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u64)]
pub enum CpuMode {
    Machine = 0b11,
    User = 0b00,
    Supervisor = 0b01,
}

pub struct Cpu {
    pub mode: CpuMode,
    pub pc: u64,
    pub regs: [u64; 32],
    pub csr: Csr,
    pub exception: Option<(Exception, u64)>,
    pub mmu: Mmu,
    pub bus: Bus,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            mode: CpuMode::Machine,
            pc: 0,
            regs: [0; 32],
            csr: Csr::new(),
            exception: None,
            mmu: Mmu::new(),
            bus: Bus::new(),
        }
    }
}
