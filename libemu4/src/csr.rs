use bitfield_macro::bitfield;
use num_enum::TryFromPrimitive;

use crate::CpuMode;

#[derive(TryFromPrimitive, PartialEq, Eq)]
#[repr(u64)]
pub enum CsrName {
    Cycle = 0xC00,
    Time = 0xC01,
    Instret = 0xC02,
    SStatus = 0x100,
    SInterruptEnable = 0x104,
    STrapHandlerVector = 0x105,
    SCounterEnable = 0x106,
    SScratch = 0x140,
    SExceptionPC = 0x141,
    STrapCause = 0x142,
    STrapValue = 0x143,
    SInterruptPending = 0x144,
    STranslationAndProtection = 0x180,

    MVendorID = 0xF11,
    MArchitectureID = 0xF12,
    MImplementationID = 0xF13,
    MHartID = 0xF14,
    MStatus = 0x300,
    MIsaAndExtensions = 0x301,
    MExceptionDelegation = 0x302,
    MInterruptDelegation = 0x303,
    MInterruptEnable = 0x304,
    MTrapHandlerVector = 0x305,
    MCounterEnable = 0x306,
    MScratch = 0x340,
    MExceptionPC = 0x341,
    MTrapCause = 0x342,
    MTrapValue = 0x343,
    MInterruptPending = 0x344,
    MCounterInhibit = 0x320,
    MEnvConfig = 0x30A,
    TSelect = 0x7A0,
    TInfo = 0x7A4,
}

#[derive(PartialEq)]
enum AccessType {
    MachineReadWrite,
    SupervisorReadWrite,
    AnyReadWrite,
    ReadOnly,
}

impl AccessType {
    fn get(csr_addr: u64) -> Option<Self> {
        let acc = match csr_addr {
            // ──────────────────────────────────────  Unprivileged/User Level CSRs ──────────────────────────────────────────────
            0x000..=0x0FF => Self::AnyReadWrite,
            0x400..=0x4FF => Self::AnyReadWrite,
            0x800..=0x8FF => Self::AnyReadWrite,
            0xC00..=0xC7F => Self::ReadOnly,
            0xC80..=0xCBF => Self::ReadOnly,
            0xCC0..=0xCFF => Self::ReadOnly,
            // ─────────────────────────────────────────  Supervisor Level CSRs ──────────────────────────────────────────────────
            0x100..=0x1FF => Self::SupervisorReadWrite,
            0x500..=0x57F => Self::SupervisorReadWrite,
            0x580..=0x5BF => Self::SupervisorReadWrite,
            0x5C0..=0x5FF => Self::SupervisorReadWrite,
            0x900..=0x97F => Self::SupervisorReadWrite,
            0x980..=0x9BF => Self::SupervisorReadWrite,
            0x9C0..=0x9FF => Self::SupervisorReadWrite,
            0xD00..=0xD7F => Self::ReadOnly,
            0xD80..=0xDBF => Self::ReadOnly,
            0xDC0..=0xDFF => Self::ReadOnly,
            // ───────────────────────────────────────────  Machine Level CSRs ───────────────────────────────────────────────────
            0x300..=0x3FF => Self::MachineReadWrite,
            0x700..=0x77F => Self::MachineReadWrite,
            0x780..=0x79F => Self::MachineReadWrite,
            0x7A0..=0x7AF => Self::MachineReadWrite,
            0x7C0..=0x7FF => Self::MachineReadWrite,
            0xB00..=0xB7F => Self::MachineReadWrite,
            0xB80..=0xBBF => Self::MachineReadWrite,
            0xBC0..=0xBFF => Self::MachineReadWrite,
            0xF00..=0xF7F => Self::ReadOnly,
            0xF80..=0xFBF => Self::ReadOnly,
            0xFC0..=0xFFF => Self::ReadOnly,
            _ => return None,
        };

        Some(acc)
    }
}

bitfield! {
    pub struct SStatus<u64> {
        pub sie: 1,
        pub spie: 5,
        pub ube: 6,
        pub spp: 8,
        pub vs: 9..=10,
        pub fs: 13..=14,
        pub xs: 15..=16,
        pub sum: 18,
        pub mxr: 19,
        pub spelp: 23,
        pub sdt: 24,
        pub uxl: 32..=33,
        pub sd: 63
    }
}

bitfield! {
    pub struct STranslationAndProtection<u64> {
        pub ppn: 0..=43,
        pub asid: 44..=59,
        pub mode: 60..=63,
    }
}

bitfield! {
    pub struct STrapHandlerVector<u64> {
        pub mode: 0..=1,
        pub base: 2..=63,
    }
}

bitfield! {
    pub struct STrapCause<u64> {
        pub exception_code: 0..=62,
        pub interrupt: 63
    }
}

bitfield! {
    pub struct MIsaAndExtensions<u64> {
        pub a_ext: 0,
        pub b_ext: 1,
        pub c_ext: 2,
        pub d_ext: 3,
        pub e_ext: 4,
        pub f_ext: 5,
        pub h_ext: 7,
        pub i_ext: 8,
        pub m_ext: 12,
        pub s_mode: 18,
        pub u_mode: 20,
        pub mxlen: 62..=63
    }
}

bitfield! {
    pub struct MStatus<u64> {
        pub sie: 1,
        pub mie: 3,
        pub spie: 5,
        pub ube: 6,
        pub mpie: 7,
        pub spp: 8,
        pub vs: 9..=10,
        pub mpp: 11..=12,
        pub fs: 13..=14,
        pub xs: 15..=16,
        pub mprv: 17,
        pub sum: 18,
        pub mxr: 19,
        pub tvm: 20,
        pub tw: 21,
        pub tsr: 22,
        pub spelp: 23,
        pub sdt: 24,
        pub uxl: 32..=33,
        pub sxl: 34..=35,
        pub sbe: 36,
        pub mbe: 37,
        pub gva: 38,
        pub mpv: 39,
        pub mpelp: 41,
        pub mdt: 42,
        pub sd: 63
    }
}

bitfield! {
    pub struct MInterruptEnable<u64> {
        pub ssie: 1,
        pub msie: 3,
        pub stie: 5,
        pub mtie: 7,
        pub seie: 9,
        pub meie: 11,
        pub lcofie: 13,
    }
}

bitfield! {
    pub struct MInterruptPending<u64> {
        pub ssip: 1,
        pub msip: 3,
        pub stip: 5,
        pub mtip: 7,
        pub seip: 9,
        pub meip: 11,
        pub lcofip: 13,
    }
}

bitfield! {
    pub struct MInterruptDelegation<u64> {
        pub ss: 1,
        pub ms: 3,
        pub st: 5,
        pub mt: 7,
        pub se: 9,
        pub me: 11,
        pub lcof: 13,
    }
}

bitfield! {
    pub struct MExceptionDelegation<u64> {
        pub unused1: 0,
        pub instr_access_fault: 1,
        pub illegal_instruction: 2,
        pub breakpoint: 3,
        pub load_addr_misaligned: 4,
        pub unused2: 5,
        pub store_addr_misaligned: 6,
        pub unused3: 7,
        pub env_call_u_mode: 8,
        pub env_call_s_mode: 9,
        pub unused4: 10..=11,
        pub instr_page_fault: 12,
        pub load_page_fault: 13,
        pub unused5: 14
        pub store_page_fault: 15,
        pub unused6: 16..=63
    }
}

bitfield! {
    pub struct MTrapHandlerVector<u64> {
        pub mode: 0..=1,
        pub base: 2..=63,
    }
}

bitfield! {
    pub struct MTrapCause<u64> {
        pub exception_code: 0..=62,
        pub interrupt: 63
    }
}

#[derive(Default)]
pub struct Csr {
    cycle: u64,
    time: u64,
    instret: u64,
    stvec: u64,
    sscratch: u64,
    sepc: u64,
    scause: u64,
    stval: u64,
    satp: u64,
    scounteren: u64,

    mstatus: u64,
    mie: u64,
    mip: u64,
    mideleg: u64,
    medeleg: u64,
    mtvec: u64,
    mscratch: u64,
    mepc: u64,
    mtval: u64,
    mcause: u64,
    mcountinhibit: u64,
    mcounteren: u64,
}

impl Csr {
    pub fn new() -> Self {
        let mut mstatus = MStatus::from_bits(0);
        mstatus.set_uxl(0b10);
        mstatus.set_sxl(0b10);
        mstatus.set_sd(mstatus.fs() == 0b11 || mstatus.xs() == 0b11 || mstatus.vs() == 0b11);

        Self {
            mstatus: mstatus.as_bits(),
            ..Default::default()
        }
    }

    pub fn load(&self, csr_addr: u64, cpu_mode: CpuMode) -> Option<u64> {
        if (0xB00..=0xB1F).contains(&(csr_addr & 0xFFF)) {
            return Some(0);
        }

        let name = CsrName::try_from(csr_addr & 0xFFF).ok()?;

        let val = match name {
            CsrName::Cycle => match cpu_mode {
                CpuMode::Machine => self.cycle,
                CpuMode::Supervisor if self.mcounteren & 1 != 0 => self.cycle,
                CpuMode::User if self.mcounteren & 1 != 0 && self.scounteren & 1 != 0 => self.cycle,
                _ => return None,
            },
            CsrName::Time => match cpu_mode {
                CpuMode::Machine => self.time,
                CpuMode::Supervisor if self.mcounteren & (1 << 1) != 0 => self.time,
                CpuMode::User if self.mcounteren & (1 << 1) != 0 && self.scounteren & 1 != 0 => {
                    self.time
                }
                _ => return None,
            },
            CsrName::Instret => match cpu_mode {
                CpuMode::Machine => self.instret,
                CpuMode::Supervisor if self.mcounteren & (1 << 2) != 0 => self.instret,
                CpuMode::User if self.mcounteren & (1 << 2) != 0 && self.scounteren & 1 != 0 => {
                    self.instret
                }
                _ => return None,
            },
            CsrName::SStatus => self.mstatus & SStatus::VALID_MASK,
            CsrName::SInterruptEnable => self.mie & self.mideleg,
            CsrName::STrapHandlerVector => self.stvec,
            CsrName::SCounterEnable => self.scounteren,
            CsrName::SScratch => self.sscratch,
            CsrName::SExceptionPC => self.sepc,
            CsrName::STrapCause => self.scause,
            CsrName::STrapValue => self.stval,
            CsrName::SInterruptPending => self.mip & self.mideleg,
            CsrName::STranslationAndProtection => self.satp,

            CsrName::MVendorID => 0,
            CsrName::MArchitectureID => 5,
            CsrName::MImplementationID => 0,
            CsrName::MHartID => 0,
            CsrName::MStatus => self.mstatus,
            CsrName::MIsaAndExtensions => {
                let mut misa = MIsaAndExtensions::from_bits(0);
                misa.set_i_ext(true);
                misa.set_m_ext(true);
                misa.set_a_ext(true);
                misa.set_c_ext(true);
                misa.set_s_mode(true);
                misa.set_u_mode(true);
                misa.set_mxlen(2);
                misa.as_bits()
            }
            CsrName::MExceptionDelegation => self.medeleg,
            CsrName::MInterruptDelegation => self.mideleg,
            CsrName::MInterruptEnable => self.mie,
            CsrName::MTrapHandlerVector => self.mtvec,
            CsrName::MCounterEnable => self.mcounteren,
            CsrName::MScratch => self.mscratch,
            CsrName::MExceptionPC => self.mepc,
            CsrName::MTrapCause => self.mcause,
            CsrName::MTrapValue => self.mtval,
            CsrName::MInterruptPending => self.mip,
            CsrName::MCounterInhibit => self.mcountinhibit,
            CsrName::MEnvConfig => 0,
            CsrName::TSelect => 0,
            CsrName::TInfo => 0,
        };

        Some(val)
    }

    pub fn store(&mut self, csr_addr: u64, val: u64, cpu_mode: CpuMode) -> Option<()> {
        if (0xB00..=0xB1F).contains(&(csr_addr & 0xFFF)) {
            return Some(());
        }

        let name = CsrName::try_from(csr_addr & 0xFFF).ok()?;
        let acc = AccessType::get(csr_addr & 0xFFF)?;

        let valid = match acc {
            AccessType::MachineReadWrite => cpu_mode == CpuMode::Machine,
            AccessType::SupervisorReadWrite => {
                cpu_mode == CpuMode::Supervisor || cpu_mode == CpuMode::Machine
            }
            AccessType::AnyReadWrite => true,
            AccessType::ReadOnly => false,
        };

        if !valid {
            return Some(());
        }

        match name {
            CsrName::Cycle => (),
            CsrName::Time => (),
            CsrName::Instret => (),
            CsrName::SStatus => {
                self.mstatus = (self.mstatus & !SStatus::VALID_MASK) | (val & SStatus::VALID_MASK);
            }
            CsrName::SInterruptEnable => {
                self.mie = (self.mie & !self.mideleg) | (val & self.mideleg);
            }
            CsrName::STrapHandlerVector => self.stvec = val,
            CsrName::SCounterEnable => self.scounteren = val,
            CsrName::SScratch => self.sscratch = val,
            CsrName::SExceptionPC => self.sepc = val,
            CsrName::STrapCause => self.scause = val,
            CsrName::STrapValue => self.stval = val,
            CsrName::SInterruptPending => {
                self.mip = (self.mip & !self.mideleg) | (val & self.mideleg)
            }
            CsrName::STranslationAndProtection => {
                let satp = STranslationAndProtection::from_bits(val);

                match satp.mode() {
                    // Bare or Sv39
                    0x0 | 0x8 => self.satp = val,
                    _ => return Some(()),
                }
            }

            CsrName::MVendorID => (),
            CsrName::MArchitectureID => (),
            CsrName::MImplementationID => (),
            CsrName::MHartID => (),
            CsrName::MStatus => self.mstatus = val,
            CsrName::MIsaAndExtensions => (),
            CsrName::MExceptionDelegation => self.medeleg = val,
            CsrName::MInterruptDelegation => self.mideleg = val,
            CsrName::MInterruptEnable => self.mie = val,
            CsrName::MTrapHandlerVector => self.mtvec = val,
            CsrName::MCounterEnable => self.mcounteren = val,
            CsrName::MScratch => self.mscratch = val,
            CsrName::MExceptionPC => self.mepc = val,
            CsrName::MTrapCause => self.mcause = val,
            CsrName::MTrapValue => self.mtval = val,
            CsrName::MInterruptPending => self.mip = val,
            CsrName::MCounterInhibit => self.mcountinhibit = val,
            CsrName::MEnvConfig => (),
            CsrName::TSelect => (),
            CsrName::TInfo => (),
        }

        self.update_mstatus(|mstatus| {
            mstatus.set_uxl(0b10);
            mstatus.set_sxl(0b10);
            mstatus.set_sd(mstatus.fs() == 0b11 || mstatus.xs() == 0b11 || mstatus.vs() == 0b11);
        });

        self.mcountinhibit &= 0b111;

        Some(())
    }

    pub fn tick(&mut self) {
        self.cycle += 1;
        self.instret += 1;
    }

    pub fn cycle(&self) -> u64 {
        self.cycle
    }

    pub fn mcountinhibit(&self) -> u64 {
        self.mcountinhibit
    }

    pub fn mstatus(&self) -> MStatus {
        MStatus::from_bits(self.mstatus)
    }

    pub fn update_mstatus(&mut self, f: impl Fn(&mut MStatus)) {
        let mut mstatus = MStatus::from_bits(self.mstatus);
        f(&mut mstatus);
        self.mstatus = mstatus.as_bits();
    }

    pub fn mip(&self) -> MInterruptPending {
        MInterruptPending::from_bits(self.mip)
    }

    pub fn update_mip(&mut self, f: impl Fn(&mut MInterruptPending)) {
        let mut mip = MInterruptPending::from_bits(self.mip);
        f(&mut mip);
        self.mip = mip.as_bits();
    }

    pub fn mie(&self) -> MInterruptEnable {
        MInterruptEnable::from_bits(self.mie)
    }

    pub fn mideleg(&self) -> MInterruptDelegation {
        MInterruptDelegation::from_bits(self.mideleg)
    }

    pub fn mepc(&self) -> u64 {
        self.mepc
    }

    pub fn set_mepc(&mut self, mepc: u64) {
        self.mepc = mepc;
    }

    pub fn mtvec(&self) -> MTrapHandlerVector {
        MTrapHandlerVector::from_bits(self.mtvec)
    }

    pub fn mcause(&self) -> u64 {
        self.mcause
    }

    pub fn set_mcause(&mut self, mcause: MTrapCause) {
        self.mcause = mcause.as_bits();
    }

    pub fn mtval(&self) -> u64 {
        self.mtval
    }

    pub fn set_mtval(&mut self, mtval: u64) {
        self.mtval = mtval;
    }

    pub fn medeleg(&self) -> MExceptionDelegation {
        MExceptionDelegation::from_bits(self.medeleg)
    }

    pub fn time(&self) -> u64 {
        self.time
    }

    pub fn set_time(&mut self, time: u64) {
        self.time = time;
    }

    pub fn sepc(&self) -> u64 {
        self.sepc
    }

    pub fn set_sepc(&mut self, sepc: u64) {
        self.sepc = sepc;
    }

    pub fn sstatus(&self) -> SStatus {
        SStatus::from_bits(self.mstatus)
    }

    pub fn update_sstatus(&mut self, f: impl Fn(&mut SStatus)) {
        let mut sstatus = SStatus::from_bits(self.mstatus);
        f(&mut sstatus);
        self.mstatus =
            (self.mstatus & !SStatus::VALID_MASK) | (sstatus.as_bits() & SStatus::VALID_MASK)
    }

    pub fn satp(&self) -> STranslationAndProtection {
        STranslationAndProtection::from_bits(self.satp)
    }

    pub fn stvec(&self) -> STrapHandlerVector {
        STrapHandlerVector::from_bits(self.stvec)
    }

    pub fn scause(&self) -> u64 {
        self.scause
    }

    pub fn set_scause(&mut self, scause: STrapCause) {
        self.scause = scause.as_bits();
    }

    pub fn stval(&self) -> u64 {
        self.stval
    }

    pub fn set_stval(&mut self, stval: u64) {
        self.stval = stval;
    }
}
