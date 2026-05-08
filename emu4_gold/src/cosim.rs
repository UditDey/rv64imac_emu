use libemu4::{
    CpuMode,
    bus::{Bus, BusTxSize},
    csr::Csr,
    decode::Instr,
    devices::{clint::Clint, plic::Plic, ram::Ram, syscon::Syscon, uart::Uart},
    payload::{BOOT_VECTOR, DEVICE_TREE_ADDR},
};

#[repr(C)]
struct TickInfo {
    time: u64,
    mtip: u8,
    bus: *mut u8,
    addr_to_mem: extern "C" fn(*mut u8, u64) -> *mut u8,
    mmio_load: extern "C" fn(*mut u8, u64, u64, *mut u8),
    mmio_store: extern "C" fn(*mut u8, u64, u64, *const u8),
}

#[repr(C)]
struct MemLog {
    load_addr: u64,
    load_size: u64,
    store_addr: u64,
    store_val: u64,
    store_size: u64,
}

#[repr(C)]
struct CosimState {
    pc: u64,
    prv: u64,
    regs: [u64; 32],
    sepc: u64,
    satp: u64,
    stval: u64,
    stvec: u64,
    scause: u64,
    sstatus: u64,
    mtval: u64,
    mcause: u64,
    mstatus: u64,
    mcountinhibit: u64,
    load_reservation: u64,
}

unsafe extern "C" {
    fn cosim_init(boot_vector: u64, dt_addr: u64);
    fn cosim_tick(tick_info: *mut TickInfo) -> MemLog;
    fn cosim_state() -> CosimState;
}

extern "C" fn addr_to_mem(bus: *mut u8, addr: u64) -> *mut u8 {
    unsafe { bus.cast::<CosimBus>().as_mut().unwrap().ram_ptr(addr) }
}

extern "C" fn mmio_load(bus: *mut u8, addr: u64, len: u64, bytes: *mut u8) {
    unsafe {
        let tx_size = match len {
            1 => BusTxSize::Bits8,
            2 => BusTxSize::Bits16,
            4 => BusTxSize::Bits32,
            8 => BusTxSize::Bits64,
            _ => panic!(),
        };

        let val = bus
            .cast::<CosimBus>()
            .as_mut()
            .unwrap()
            .mmio_load(addr, tx_size)
            .unwrap();

        match tx_size {
            BusTxSize::Bits8 => bytes.write(val as u8),
            BusTxSize::Bits16 => bytes.cast::<u16>().write(val as u16),
            BusTxSize::Bits32 => bytes.cast::<u32>().write(val as u32),
            BusTxSize::Bits64 => bytes.cast::<u64>().write(val),
        }
    }
}

extern "C" fn mmio_store(bus: *mut u8, addr: u64, len: u64, bytes: *const u8) {
    unsafe {
        let tx_size = match len {
            1 => BusTxSize::Bits8,
            2 => BusTxSize::Bits16,
            4 => BusTxSize::Bits32,
            8 => BusTxSize::Bits64,
            _ => panic!(),
        };

        let val = match tx_size {
            BusTxSize::Bits8 => bytes.read() as u64,
            BusTxSize::Bits16 => bytes.cast::<u16>().read() as u64,
            BusTxSize::Bits32 => bytes.cast::<u32>().read() as u64,
            BusTxSize::Bits64 => bytes.cast::<u64>().read(),
        };

        bus.cast::<CosimBus>()
            .as_mut()
            .unwrap()
            .mmio_store(addr, val, tx_size);
    }
}

struct CosimBus {
    ram: Ram,
    syscon: Syscon,
    clint: Clint,
    plic: Plic,
    uart: Uart,
}

impl CosimBus {
    fn new() -> Self {
        Self {
            ram: Ram::new(),
            syscon: Syscon::new(),
            clint: Clint::new(),
            plic: Plic::new(),
            uart: Uart::new(),
        }
    }

    fn tick(&mut self, delta: u64) -> bool {
        self.uart.tick(&mut self.plic);
        self.clint.tick_cosim(delta)
    }

    fn ram_ptr(&mut self, addr: u64) -> *mut u8 {
        if Ram::ADDR_RANGE.contains(&addr) {
            &raw mut self.ram.0[(addr - Ram::BASE_ADDR) as usize]
        } else {
            std::ptr::null_mut()
        }
    }

    fn mmio_load(&mut self, addr: u64, tx_size: BusTxSize) -> Option<u64> {
        if Syscon::ADDR_RANGE.contains(&addr) {
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

    fn mmio_store(&mut self, addr: u64, val: u64, tx_size: BusTxSize) -> Option<()> {
        if Syscon::ADDR_RANGE.contains(&addr) {
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
}

pub struct Cosim {
    cosim_bus: CosimBus,
    load: Option<(u64, BusTxSize)>,
    store: Option<(u64, u64, BusTxSize)>,
}

impl Cosim {
    pub fn new(bus: &Bus) -> Self {
        let mut cosim_bus = CosimBus::new();
        cosim_bus.ram.0.copy_from_slice(&bus.ram.0);

        unsafe {
            cosim_init(BOOT_VECTOR, DEVICE_TREE_ADDR);
        }

        Self {
            cosim_bus,
            load: None,
            store: None,
        }
    }

    pub fn tick(&mut self, csr: &Csr, delta: u64) {
        let mtip = self.cosim_bus.tick(delta);
        self.load = None;
        self.store = None;

        //let mtip = csr.mip().mtip();
        if mtip {
            //println!("cosim: mtip high");
        }

        let mut tick_info = TickInfo {
            time: csr.time(),
            mtip: mtip as u8,
            bus: &raw mut self.cosim_bus as *mut u8,
            addr_to_mem,
            mmio_load,
            mmio_store,
        };

        let mem_log = unsafe { cosim_tick(&mut tick_info) };

        if mem_log.load_size != 0 {
            let tx_size = match mem_log.load_size {
                1 => BusTxSize::Bits8,
                2 => BusTxSize::Bits16,
                4 => BusTxSize::Bits32,
                8 => BusTxSize::Bits64,
                _ => panic!(),
            };

            self.load = Some((mem_log.load_addr & 0xFFFF_FFFF, tx_size));
        }

        if mem_log.store_size != 0 {
            let tx_size = match mem_log.store_size {
                1 => BusTxSize::Bits8,
                2 => BusTxSize::Bits16,
                4 => BusTxSize::Bits32,
                8 => BusTxSize::Bits64,
                _ => panic!(),
            };

            self.store = Some((mem_log.store_addr & 0xFFFF_FFFF, mem_log.store_val, tx_size));
        }
    }

    pub fn compare_states(
        &self,
        pc: u64,
        cpu_mode: CpuMode,
        regs: &[u64; 32],
        csr: &Csr,
        bus: &Bus,
        instr: &Option<Instr>,
    ) {
        let cosim_state = unsafe { cosim_state() };
        let mut diffs = vec![];

        if pc != cosim_state.pc {
            diffs.push(format!("PC: 0x{:X} vs 0x{:X}", pc, cosim_state.pc));
        }

        if cpu_mode as u64 != cosim_state.prv {
            diffs.push(format!(
                "PRV: {:?} vs {:?}",
                cpu_mode,
                CpuMode::try_from(cosim_state.prv).unwrap(),
            ));
        }

        for (i, (reg, cosim_reg)) in regs.iter().zip(&cosim_state.regs).enumerate() {
            if reg != cosim_reg {
                diffs.push(format!("Regs[{}]: 0x{:X} vs 0x{:X}", i, reg, cosim_reg));
            }
        }

        if csr.sepc() != cosim_state.sepc {
            diffs.push(format!(
                "SEPC: 0x{:X} vs 0x{:X}",
                csr.sepc(),
                cosim_state.sepc
            ));
        }

        if csr.satp().as_bits() != cosim_state.satp {
            diffs.push(format!(
                "SATP: 0x{:X} vs 0x{:X}",
                csr.satp().as_bits(),
                cosim_state.satp
            ));
        }

        if csr.stval() != cosim_state.stval {
            diffs.push(format!(
                "STVAL: 0x{:X} vs 0x{:X}",
                csr.stval(),
                cosim_state.stval
            ));
        }

        if csr.stvec().as_bits() != cosim_state.stvec {
            diffs.push(format!(
                "STVEC: 0x{:X} vs 0x{:X}",
                csr.stvec().as_bits(),
                cosim_state.stvec
            ));
        }

        if csr.scause() != cosim_state.scause {
            diffs.push(format!(
                "SCAUSE: 0x{:X} vs 0x{:X}",
                csr.scause(),
                cosim_state.scause
            ));
        }

        if csr.sstatus().as_bits() != cosim_state.sstatus {
            diffs.push(format!(
                "SSTATUS: 0x{:X} vs 0x{:X}",
                csr.sstatus().as_bits(),
                cosim_state.sstatus
            ));
        }

        if csr.mtval() != cosim_state.mtval {
            diffs.push(format!(
                "MTVAL: 0x{:X} vs 0x{:X}",
                csr.mtval(),
                cosim_state.mtval
            ));
        }

        if csr.mcause() != cosim_state.mcause {
            diffs.push(format!(
                "MCAUSE: 0x{:X} vs 0x{:X}",
                csr.mcause(),
                cosim_state.mcause
            ));
        }

        if csr.mstatus().as_bits() != cosim_state.mstatus {
            diffs.push(format!(
                "MSTATUS: 0x{:X} vs 0x{:X}",
                csr.mstatus().as_bits(),
                cosim_state.mstatus
            ));
        }

        if csr.mcountinhibit() != cosim_state.mcountinhibit {
            diffs.push(format!(
                "MCOUNTINHIBIT: 0x{:X} vs 0x{:X}",
                csr.mcountinhibit(),
                cosim_state.mcountinhibit
            ));
        }

        if bus.last_load != self.load {
            diffs.push(format!(
                "Bus Load: {:X?} vs {:X?}",
                bus.last_load, self.load
            ));
        }

        if bus.pending_store != self.store {
            diffs.push(format!(
                "Bus Store: {:X?} vs {:X?}",
                bus.pending_store, self.store
            ));
        }

        let cosim_reservation = if cosim_state.load_reservation != u64::MAX {
            Some(cosim_state.load_reservation)
        } else {
            None
        };

        if bus.atomic_reservation != cosim_reservation {
            diffs.push(format!(
                "Load Reservation: {:X?} vs 0x{:X}",
                bus.atomic_reservation, cosim_state.load_reservation
            ));
        }

        if csr.cycle() == 664875315 && bus.ram.0 != self.cosim_bus.ram.0 {
            panic!("ram mismatch");
        }

        if !diffs.is_empty() {
            panic!(
                "Diffs:\n{diffs:#?}\nLast instr: {instr:?} in cycle {}\nRegs: {regs:#?}",
                csr.cycle(),
            );
        }
    }
}
