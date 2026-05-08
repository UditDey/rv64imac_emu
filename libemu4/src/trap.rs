use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{
    CpuMode,
    csr::{Csr, MTrapCause, STrapCause},
};

#[derive(IntoPrimitive, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u64)]
pub enum Exception {
    InstructionAddressMisaligned = 0,
    InstructionAccessFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadAddressMisaligned = 4,
    LoadAccessFault = 5,
    StoreAddressMisaligned = 6,
    StoreAccessFault = 7,
    EnvCallUMode = 8,
    EnvCallSMode = 9,
    EnvCallMMode = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
}

#[derive(TryFromPrimitive)]
#[repr(u64)]
enum TrapHandlerMode {
    Direct = 0,
    Vectored = 1,
}

#[derive(Debug, PartialEq)]
enum InterruptType {
    MachineExternal,
    MachineSoftware,
    MachineTimer,
    SupervisorExternal,
    SupervisorSoftware,
    SupervisorTimer,
}

impl InterruptType {
    fn value(&self) -> u64 {
        match self {
            InterruptType::MachineExternal => 11,
            InterruptType::MachineSoftware => 3,
            InterruptType::MachineTimer => 7,
            InterruptType::SupervisorExternal => 9,
            InterruptType::SupervisorSoftware => 1,
            InterruptType::SupervisorTimer => 5,
        }
    }
}

pub fn process_interrupt(csr: &mut Csr, cpu_mode: &mut CpuMode, pc: &mut u64) -> bool {
    let mip = csr.mip();
    let mie = csr.mie();
    let mideleg = csr.mideleg();
    let mstatus = csr.mstatus();

    for i in 0..6 {
        let int_type = match i {
            0 => InterruptType::MachineExternal,
            1 => InterruptType::MachineSoftware,
            2 => InterruptType::MachineTimer,
            3 => InterruptType::SupervisorExternal,
            4 => InterruptType::SupervisorSoftware,
            5 => InterruptType::SupervisorTimer,
            _ => unreachable!(),
        };

        let pending = match int_type {
            InterruptType::MachineExternal => mip.meip() && mie.meie(),
            InterruptType::MachineSoftware => mip.msip() && mie.msie(),
            InterruptType::MachineTimer => mip.mtip() && mie.mtie(),
            InterruptType::SupervisorExternal => mip.seip() && mie.seie(),
            InterruptType::SupervisorSoftware => mip.ssip() && mie.ssie(),
            InterruptType::SupervisorTimer => mip.stip() && mie.stie(),
        };

        if !pending {
            continue;
        }

        let delegated = match int_type {
            InterruptType::MachineExternal => mideleg.me(),
            InterruptType::MachineSoftware => mideleg.ms(),
            InterruptType::MachineTimer => mideleg.mt(),
            InterruptType::SupervisorExternal => mideleg.se(),
            InterruptType::SupervisorSoftware => mideleg.ss(),
            InterruptType::SupervisorTimer => mideleg.st(),
        };

        let target_is_supervisor = delegated;
        let global_enabled = match (*cpu_mode, target_is_supervisor) {
            (CpuMode::User, true) => true,                // U < S
            (CpuMode::Supervisor, true) => mstatus.sie(), // S == S
            (CpuMode::Machine, true) => false,            // M > S  (ALWAYS OFF)
            (CpuMode::Machine, false) => mstatus.mie(),   // M == M
            (_, false) => true,                           // S/U < M
        };

        if !global_enabled {
            continue;
        }

        if delegated {
            //println!("emu4: Handling {int_type:?} in supervisor mode");
            // Handle in supervisor mode
            csr.update_sstatus(|sstatus| {
                sstatus.set_spp(*cpu_mode == CpuMode::Supervisor); // sstatus.spp stores cpu mode prior to interrupt
                sstatus.set_spie(sstatus.sie()); // sstatus.spie stores sstatus.sie prior to interrupt
                sstatus.set_sie(false); // sstatus.sie disabled while current interrupt is being handled
            });

            *cpu_mode = CpuMode::Supervisor;
            csr.set_sepc(*pc & !1); // sepc stores PC prior to interrupt (so it can `iret` back to it)

            // Load trap handler addr and update PC to it
            let stvec = csr.stvec();
            let stvec_mode: TrapHandlerMode = stvec.mode().try_into().unwrap();

            *pc = match stvec_mode {
                TrapHandlerMode::Direct => stvec.base() << 2,
                TrapHandlerMode::Vectored => (stvec.base() << 2) + int_type.value() * 4,
            };

            // scause stores interrupt type
            let mut scause = STrapCause::from_bits(0);
            scause.set_exception_code(int_type.value());
            scause.set_interrupt(true);
            csr.set_scause(scause);

            // stval stores exception trap related value (nothing for interrupts)
            csr.set_stval(0);
        } else {
            // Handle in machine mode
            csr.update_mstatus(|mstatus| {
                mstatus.set_mpp(*cpu_mode as u64);
                mstatus.set_mpie(mstatus.mie()); // mstatus.mpie stores mstatus.mie prior to interrupt
                mstatus.set_mie(false); // mstatus.sie disabled while current interrupt is being handled
            });

            *cpu_mode = CpuMode::Machine;
            csr.set_mepc(*pc & !1); // mepc stores PC prior to interrupt (so it can `iret` back to it)

            // Load trap handler addr and update PC to it
            let mtvec = csr.mtvec();
            let mtvec_mode: TrapHandlerMode = mtvec.mode().try_into().unwrap();

            *pc = match mtvec_mode {
                TrapHandlerMode::Direct => mtvec.base() << 2,
                TrapHandlerMode::Vectored => (mtvec.base() << 2) + int_type.value() * 4,
            };
            /*println!(
                "emu4: Handling {int_type:?} in machine mode, jumping to 0x{:X}",
                pc
            );*/

            // mcause stores interrupt type
            let mut mcause = MTrapCause::from_bits(0);
            mcause.set_exception_code(int_type.value());
            mcause.set_interrupt(true);
            csr.set_mcause(mcause);

            // mtval stores exception trap related value (nothing for interrupts)
            csr.set_mtval(0);
        }

        return true;
    }

    false
}

pub fn process_exception(
    exception: Exception,
    exception_data: u64,
    csr: &mut Csr,
    cpu_mode: &mut CpuMode,
    pc: &mut u64,
) {
    // Check if the exception should be delegated to supervisor
    let medeleg = csr.medeleg();

    let delegate = match exception {
        Exception::IllegalInstruction => medeleg.illegal_instruction(),
        Exception::Breakpoint => medeleg.breakpoint(),
        Exception::EnvCallSMode => medeleg.env_call_s_mode(),
        Exception::InstructionPageFault => medeleg.instr_page_fault(),
        Exception::InstructionAccessFault => medeleg.instr_access_fault(),
        Exception::LoadAddressMisaligned => medeleg.load_addr_misaligned(),
        Exception::StoreAddressMisaligned => medeleg.store_addr_misaligned(),
        Exception::StorePageFault => medeleg.store_page_fault(),
        Exception::LoadPageFault => medeleg.load_page_fault(),
        Exception::EnvCallUMode => medeleg.env_call_u_mode(),
        _ => panic!("{exception:?}"),
    };

    // Handle exception
    if *cpu_mode != CpuMode::Machine && delegate {
        //println!("emu4: Handling {exception:?} in supervisor mode");
        // Handle in supervisor mode
        csr.update_sstatus(|sstatus| {
            sstatus.set_spp(*cpu_mode == CpuMode::Supervisor); // sstatus.spp stores cpu mode prior to interrupt
            sstatus.set_spie(sstatus.sie()); // sstatus.spie stores sstatus.sie prior to interrupt
            sstatus.set_sie(false); // sstatus.sie disabled while current interrupt is being handled
        });

        *cpu_mode = CpuMode::Supervisor;
        csr.set_sepc(*pc); // sepc stores PC prior to interrupt (so it can `iret` back to it)

        // Load trap handler addr and update PC to it
        *pc = csr.stvec().base() << 2;

        // scause stores interrupt type
        let mut scause = STrapCause::from_bits(0);
        scause.set_exception_code(exception as u64);
        csr.set_scause(scause);

        // stval stores additional exception data
        csr.set_stval(exception_data);
    } else {
        //println!("emu4: Handling {exception:?} in machine mode");
        // Handle in machine mode
        csr.update_mstatus(|mstatus| {
            mstatus.set_mpp(*cpu_mode as u64);
            mstatus.set_mpie(mstatus.mie()); // mstatus.mpie stores mstatus.mie prior to interrupt
            mstatus.set_mie(false); // mstatus.sie disabled while current interrupt is being handled
        });

        *cpu_mode = CpuMode::Machine;
        csr.set_mepc(*pc); // mepc stores PC prior to interrupt (so it can `iret` back to it)

        // Load trap handler addr and update PC to it
        *pc = csr.mtvec().base() << 2;

        // mcause stores interrupt type
        let mut mcause = MTrapCause::from_bits(0);
        mcause.set_exception_code(exception as u64);
        csr.set_mcause(mcause);

        // mtval stores exception trap related value (nothing for interrupts)
        csr.set_mtval(exception_data);
    }
}
