use crate::{
    Cpu, CpuMode,
    bus::BusTxSize,
    decode::{Instr, InstrFlow, InstrOp, OpWidth},
    mmu::AccessType,
    trap::Exception,
};

fn binary_op(cpu: &Cpu, instr: &Instr) -> (u64, u64, usize) {
    match instr.flow {
        InstrFlow::RegReg2Reg { rs_1, rs_2, rd } => (cpu.regs[rs_1], cpu.regs[rs_2], rd),
        InstrFlow::RegImm2Reg { rs_1, imm, rd } => (cpu.regs[rs_1], imm, rd),
        _ => panic!(),
    }
}

fn csr_op(cpu: &Cpu, instr: &Instr) -> (u64, usize, u64) {
    match instr.flow {
        InstrFlow::Reg2RegCsr { rs_1, rd, csr } => (cpu.regs[rs_1], rd, csr),
        InstrFlow::Imm2RegCsr { imm, rd, csr } => (imm, rd, csr),
        _ => panic!(),
    }
}

fn translate_addr(cpu: &mut Cpu, virt_addr: u64, acc_type: AccessType) -> Result<u64, Exception> {
    cpu.mmu
        .translate(virt_addr, acc_type, cpu.mode, &cpu.csr, &mut cpu.bus)
}

pub fn exec(cpu: &mut Cpu, instr: &Instr, instr_raw: u32) {
    match instr.op {
        InstrOp::LoadUpperImm => {
            let InstrFlow::Imm2Reg { imm, rd } = instr.flow else {
                panic!()
            };

            cpu.regs[rd] = imm;
        }
        InstrOp::AddUpperImmAndPC => {
            let InstrFlow::ImmPC2Reg { imm, rd } = instr.flow else {
                panic!()
            };

            cpu.regs[rd] = cpu.pc + imm;
        }

        InstrOp::JumpAndLink => {
            let InstrFlow::ImmPC2RegPC { imm, rd } = instr.flow else {
                panic!()
            };

            cpu.regs[rd] = cpu.pc + instr.instr_size as u64;
            cpu.pc += imm - instr.instr_size as u64;
        }
        InstrOp::JumpAndLinkReg => {
            let InstrFlow::RegImmPC2RegPC { rs_1, imm, rd } = instr.flow else {
                panic!()
            };

            let rs_1 = cpu.regs[rs_1];
            cpu.regs[rd] = cpu.pc + instr.instr_size as u64;
            cpu.pc = ((rs_1 + imm) & !1) - instr.instr_size as u64;
        }

        InstrOp::BranchIfEquals => {
            let InstrFlow::RegRegImm2PC { rs_1, rs_2, imm } = instr.flow else {
                panic!()
            };

            if cpu.regs[rs_1] == cpu.regs[rs_2] {
                cpu.pc += imm - instr.instr_size as u64;
            }
        }
        InstrOp::BranchIfNotEquals => {
            let InstrFlow::RegRegImm2PC { rs_1, rs_2, imm } = instr.flow else {
                panic!()
            };

            if cpu.regs[rs_1] != cpu.regs[rs_2] {
                cpu.pc += imm - instr.instr_size as u64;
            }
        }
        InstrOp::BranchIfLesserThan => {
            let InstrFlow::RegRegImm2PC { rs_1, rs_2, imm } = instr.flow else {
                panic!()
            };

            let branch = match instr.op_width {
                OpWidth::DoubleWord => (cpu.regs[rs_1] as i64) < (cpu.regs[rs_2] as i64),
                OpWidth::DoubleWordUnsigned => cpu.regs[rs_1] < cpu.regs[rs_2],
                _ => panic!(),
            };

            if branch {
                cpu.pc += imm - instr.instr_size as u64;
            }
        }
        InstrOp::BranchIfGreaterThanOrEquals => {
            let InstrFlow::RegRegImm2PC { rs_1, rs_2, imm } = instr.flow else {
                panic!()
            };

            let branch = match instr.op_width {
                OpWidth::DoubleWord => cpu.regs[rs_1] as i64 >= cpu.regs[rs_2] as i64,
                OpWidth::DoubleWordUnsigned => cpu.regs[rs_1] >= cpu.regs[rs_2],
                _ => panic!(),
            };

            if branch {
                cpu.pc += imm - instr.instr_size as u64;
            }
        }

        InstrOp::Load => {
            let InstrFlow::RegImmMem2Reg { rs_1, imm, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1] + imm, AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1] + imm));
                    return;
                }
            };

            let misaligned = match instr.op_width {
                OpWidth::DoubleWord => addr % 8 != 0,
                OpWidth::Word | OpWidth::WordUnsigned => addr % 4 != 0,
                OpWidth::HalfWord | OpWidth::HalfWordUnsigned => addr % 2 != 0,
                OpWidth::Byte | OpWidth::ByteUnsigned => false,
                _ => panic!(),
            };

            if misaligned {
                cpu.exception = Some((Exception::LoadAddressMisaligned, cpu.regs[rs_1] + imm));
                return;
            }

            let tx_size = match instr.op_width {
                OpWidth::DoubleWord => BusTxSize::Bits64,
                OpWidth::Word | OpWidth::WordUnsigned => BusTxSize::Bits32,
                OpWidth::HalfWord | OpWidth::HalfWordUnsigned => BusTxSize::Bits16,
                OpWidth::Byte | OpWidth::ByteUnsigned => BusTxSize::Bits8,
                _ => panic!(),
            };

            let val = cpu
                .bus
                .load(addr, tx_size)
                .unwrap_or_else(|| panic!("0x{addr:X}"));

            cpu.regs[rd] = match instr.op_width {
                OpWidth::Word => val as u32 as i32 as i64 as u64,
                OpWidth::HalfWord => val as u16 as i16 as i64 as u64,
                OpWidth::Byte => val as u8 as i8 as i64 as u64,
                OpWidth::DoubleWord
                | OpWidth::WordUnsigned
                | OpWidth::HalfWordUnsigned
                | OpWidth::ByteUnsigned => val,
                _ => panic!(),
            };
        }

        InstrOp::Store => {
            let InstrFlow::RegRegImm2Mem { rs_1, rs_2, imm } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1] + imm, AccessType::Store) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1] + imm));
                    return;
                }
            };

            let misaligned = match instr.op_width {
                OpWidth::DoubleWord => addr % 8 != 0,
                OpWidth::Word | OpWidth::WordUnsigned => addr % 4 != 0,
                OpWidth::HalfWord | OpWidth::HalfWordUnsigned => addr % 2 != 0,
                OpWidth::Byte | OpWidth::ByteUnsigned => false,
                _ => panic!(),
            };

            if misaligned {
                cpu.exception = Some((Exception::StoreAddressMisaligned, cpu.regs[rs_1] + imm));
                return;
            }

            let tx_size = match instr.op_width {
                OpWidth::DoubleWord => BusTxSize::Bits64,
                OpWidth::Word => BusTxSize::Bits32,
                OpWidth::HalfWord => BusTxSize::Bits16,
                OpWidth::Byte => BusTxSize::Bits8,
                _ => panic!(),
            };

            cpu.bus.store(addr, cpu.regs[rs_2], tx_size);
        }

        InstrOp::Add => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => op_1 + op_2,
                OpWidth::Word => (op_1 as u32 + op_2 as u32) as i32 as i64 as u64,
                _ => panic!(),
            };
        }
        InstrOp::Multiply => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => op_1 * op_2,
                OpWidth::Word => (op_1 as u32 * op_2 as u32) as i32 as i64 as u64,
                _ => panic!(),
            };
        }
        InstrOp::Subtract => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => op_1 - op_2,
                OpWidth::Word => (op_1 as u32 - op_2 as u32) as i32 as i64 as u64,
                _ => panic!(),
            };
        }
        InstrOp::ShiftLeft => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => op_1 << (op_2 & 0b11_1111),
                OpWidth::Word => ((op_1 as u32) << (op_2 & 0b1_1111)) as i32 as i64 as u64,
                _ => panic!(),
            };
        }
        InstrOp::MultiplyHigh => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => {
                    ((op_1 as i64 as i128 * op_2 as i64 as i128) as u128 >> 64) as u64
                }
                OpWidth::DoubleWordSignedUnsigned => {
                    ((op_1 as i64 as i128 * op_2 as u128 as i128) as u128 >> 64) as u64
                }
                OpWidth::DoubleWordUnsigned => ((op_1 as u128 * op_2 as u128) >> 64) as u64,
                _ => panic!(),
            };
        }
        InstrOp::SetLesserThan => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => ((op_1 as i64) < (op_2 as i64)) as u64,
                OpWidth::DoubleWordUnsigned => (op_1 < op_2) as u64,
                _ => panic!(),
            };
        }
        InstrOp::Xor => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);
            cpu.regs[rd] = op_1 ^ op_2;
        }
        InstrOp::Divide => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => {
                    let (op_1, op_2) = (op_1 as i64, op_2 as i64);

                    match (op_1, op_2) {
                        (_, 0) => !0,
                        (i64::MIN, -1) => op_1 as u64,
                        (op_1, op_2) => (op_1 / op_2) as u64,
                    }
                }
                OpWidth::Word => {
                    let (op_1, op_2) = (op_1 as u32 as i32, op_2 as u32 as i32);

                    match (op_1, op_2) {
                        (_, 0) => !0,
                        (i32::MIN, -1) => op_1 as u32 as u64,
                        (op_1, op_2) => (op_1 / op_2) as i64 as u64,
                    }
                }
                OpWidth::DoubleWordUnsigned => match (op_1, op_2) {
                    (_, 0) => !0,
                    (op_1, op_2) => op_1 / op_2,
                },
                OpWidth::WordUnsigned => {
                    let (op_1, op_2) = (op_1 as u32, op_2 as u32);

                    match (op_1, op_2) {
                        (_, 0) => !0,
                        (op_1, op_2) => (op_1 / op_2) as i32 as i64 as u64,
                    }
                }
                _ => panic!(),
            };
        }
        InstrOp::ShiftRightLogical => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => op_1 >> (op_2 & 0b11_1111),
                OpWidth::Word => ((op_1 as u32) >> (op_2 & 0b1_1111)) as i32 as i64 as u64,
                _ => panic!(),
            };
        }
        InstrOp::ShiftRightArithmetic => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => ((op_1 as i64) >> (op_2 & 0b11_1111)) as u64,
                OpWidth::Word => ((op_1 as u32 as i32) >> (op_2 & 0b1_1111)) as i64 as u64,
                _ => panic!(),
            };
        }
        InstrOp::Or => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);
            cpu.regs[rd] = op_1 | op_2;
        }
        InstrOp::Remainder => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => {
                    let (op_1, op_2) = (op_1 as i64, op_2 as i64);

                    match (op_1, op_2) {
                        (_, 0) => op_1 as u64,
                        (i64::MIN, -1) => 0,
                        (op_1, op_2) => (op_1 % op_2) as u64,
                    }
                }
                OpWidth::Word => {
                    let (op_1, op_2) = (op_1 as u32 as i32, op_2 as u32 as i32);

                    match (op_1, op_2) {
                        (_, 0) => op_1 as u64,
                        (i32::MIN, -1) => 0,
                        (op_1, op_2) => (op_1 % op_2) as i64 as u64,
                    }
                }
                OpWidth::DoubleWordUnsigned => match (op_1, op_2) {
                    (_, 0) => op_1,
                    (op_1, op_2) => op_1 % op_2,
                },
                OpWidth::WordUnsigned => {
                    let (op_1, op_2) = (op_1 as u32, op_2 as u32);

                    match (op_1, op_2) {
                        (_, 0) => op_1 as i32 as i64 as u64,
                        (op_1, op_2) => (op_1 % op_2) as i32 as i64 as u64,
                    }
                }
                _ => panic!(),
            };
        }
        InstrOp::And => {
            let (op_1, op_2, rd) = binary_op(cpu, instr);
            cpu.regs[rd] = op_1 & op_2;
        }

        InstrOp::FenceI => (),
        InstrOp::EnvCall => {
            let exc = match cpu.mode {
                CpuMode::Machine => Exception::EnvCallMMode,
                CpuMode::User => Exception::EnvCallUMode,
                CpuMode::Supervisor => Exception::EnvCallSMode,
            };

            cpu.exception = Some((exc, 0));
        }
        InstrOp::EnvBreak => {
            cpu.exception = Some((Exception::Breakpoint, cpu.pc));
        }
        InstrOp::SReturnFromTrap => {
            if cpu.csr.mstatus().tsr() || cpu.mode == CpuMode::User {
                cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                return;
            }

            cpu.pc = cpu.csr.sepc() - instr.instr_size as u64;
            cpu.mode = if cpu.csr.mstatus().spp() {
                CpuMode::Supervisor
            } else {
                CpuMode::User
            };

            cpu.csr.update_mstatus(|mstatus| {
                if cpu.mode == CpuMode::User {
                    mstatus.set_mprv(false);
                }
            });

            cpu.csr.update_sstatus(|sstatus| {
                sstatus.set_sie(sstatus.spie());
                sstatus.set_spie(true);
                sstatus.set_spp(false);
            });
        }
        InstrOp::MReturnFromTrap => {
            //println!("mret");
            if cpu.mode != CpuMode::Machine {
                cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                return;
            }

            cpu.pc = cpu.csr.mepc() - instr.instr_size as u64;
            cpu.mode = CpuMode::try_from(cpu.csr.mstatus().mpp()).unwrap();

            cpu.csr.update_mstatus(|mstatus| {
                //println!("mret, mstatus.mpie is {}", mstatus.mpie());
                if cpu.mode != CpuMode::Machine {
                    mstatus.set_mprv(false);
                }

                mstatus.set_mie(mstatus.mpie());
                mstatus.set_mpie(true);
                mstatus.set_mpp(CpuMode::User as u64);
            });
        }

        InstrOp::CsrReadAndWrite => {
            let (op_1, rd, csr) = csr_op(cpu, instr);
            if csr == 0x300 {
                /*println!(
                    "csrrw {op_1:b} {rd} {instr:?} (0x{instr_raw:X}) @ 0x{:X}\nregs: {:?}",
                    cpu.pc, cpu.regs,
                );*/
            }

            if rd != 0 {
                let original = match cpu.csr.load(csr, cpu.mode) {
                    Some(csr) => csr,
                    None => {
                        cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                        return;
                    }
                };

                cpu.regs[rd] = original;
            }

            if cpu.csr.store(csr, op_1, cpu.mode).is_none() {
                cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
            }
        }
        InstrOp::CsrReadAndSetBits => {
            let (op_1, rd, csr) = csr_op(cpu, instr);
            /*if csr == 0x300 {
                println!("csrrs {op_1:b} {rd}");
            }*/

            let original = match cpu.csr.load(csr, cpu.mode) {
                Some(csr) => csr,
                None => {
                    cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                    return;
                }
            };

            cpu.regs[rd] = original;

            if op_1 != 0 {
                if cpu.csr.store(csr, original | op_1, cpu.mode).is_none() {
                    cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                }
            }
        }
        InstrOp::CsrReadAndClearBits => {
            let (op_1, rd, csr) = csr_op(cpu, instr);
            /*if csr == 0x300 {
                println!("csrrc {op_1:b} {rd}");
            }*/

            let original = match cpu.csr.load(csr, cpu.mode) {
                Some(csr) => csr,
                None => {
                    cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                    return;
                }
            };

            cpu.regs[rd] = original;

            if op_1 != 0 {
                if cpu.csr.store(csr, original & !op_1, cpu.mode).is_none() {
                    cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                }
            }
        }

        InstrOp::LoadReserved => {
            let InstrFlow::RegMem2Reg { rs_1, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let tx_size = match instr.op_width {
                OpWidth::DoubleWord => BusTxSize::Bits64,
                OpWidth::Word => BusTxSize::Bits32,
                _ => panic!(),
            };

            let val = cpu.bus.load(addr, tx_size).unwrap();

            cpu.regs[rd] = match instr.op_width {
                OpWidth::DoubleWord => val,
                OpWidth::Word => val as u32 as i32 as i64 as u64,
                _ => panic!(),
            };

            /*let res_addr = match tx_size {
                BusTxSize::Bits64 => addr,
                BusTxSize::Bits32 => addr & !0x3,
                _ => panic!(),
            };*/
            //println!("emu4: lr: Reserving 0x{res_addr:X}");
            cpu.bus.atomic_reservation = Some(addr);
        }
        InstrOp::StoreConditional => {
            let InstrFlow::RegReg2RegMem { rs_1, rs_2, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Store) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let tx_size = match instr.op_width {
                OpWidth::DoubleWord => BusTxSize::Bits64,
                OpWidth::Word => BusTxSize::Bits32,
                _ => panic!(),
            };

            /*let res_addr = match tx_size {
                BusTxSize::Bits64 => addr,
                BusTxSize::Bits32 => addr & !0x3,
                _ => panic!(),
            };*/

            match cpu.bus.atomic_reservation.take() {
                Some(x) if x == addr => {
                    //println!("emu4: sc: Clearing reservation 0x{addr:X}");
                    cpu.bus.store(addr, cpu.regs[rs_2], tx_size).unwrap();
                    cpu.regs[rd] = 0;
                }
                _ => cpu.regs[rd] = 1,
            }
        }
        InstrOp::AtomicSwap => {
            let InstrFlow::RegRegMem2RegMem { rs_1, rs_2, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let original = match instr.op_width {
                OpWidth::DoubleWord => cpu.bus.load(addr, BusTxSize::Bits64).unwrap(),
                OpWidth::Word => {
                    cpu.bus.load(addr, BusTxSize::Bits32).unwrap() as u32 as i32 as i64 as u64
                }
                _ => panic!(),
            };

            let new = cpu.regs[rs_2];
            cpu.regs[rd] = original;

            match instr.op_width {
                OpWidth::DoubleWord => cpu.bus.store(addr, new, BusTxSize::Bits64).unwrap(),
                OpWidth::Word => cpu.bus.store(addr, new, BusTxSize::Bits32).unwrap(),
                _ => panic!(),
            };
        }
        InstrOp::AtomicAdd => {
            let InstrFlow::RegRegMem2RegMem { rs_1, rs_2, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let original = match instr.op_width {
                OpWidth::DoubleWord => cpu.bus.load(addr, BusTxSize::Bits64).unwrap(),
                OpWidth::Word => {
                    cpu.bus.load(addr, BusTxSize::Bits32).unwrap() as u32 as i32 as i64 as u64
                }
                _ => panic!(),
            };

            match instr.op_width {
                OpWidth::DoubleWord => {
                    let new = original + cpu.regs[rs_2];
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits64).unwrap();
                }
                OpWidth::Word => {
                    let new = (original as u32 + cpu.regs[rs_2] as u32) as i32 as i64 as u64;
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits32).unwrap();
                }
                _ => panic!(),
            };
        }
        InstrOp::AtomicXor => {
            let InstrFlow::RegRegMem2RegMem { rs_1, rs_2, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let original = match instr.op_width {
                OpWidth::DoubleWord => cpu.bus.load(addr, BusTxSize::Bits64).unwrap(),
                OpWidth::Word => {
                    cpu.bus.load(addr, BusTxSize::Bits32).unwrap() as u32 as i32 as i64 as u64
                }
                _ => panic!(),
            };

            match instr.op_width {
                OpWidth::DoubleWord => {
                    let new = original ^ cpu.regs[rs_2];
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits64).unwrap();
                }
                OpWidth::Word => {
                    let new = (original as u32 ^ cpu.regs[rs_2] as u32) as i32 as i64 as u64;
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits32).unwrap();
                }
                _ => panic!(),
            };
        }
        InstrOp::AtomicAnd => {
            let InstrFlow::RegRegMem2RegMem { rs_1, rs_2, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let original = match instr.op_width {
                OpWidth::DoubleWord => cpu.bus.load(addr, BusTxSize::Bits64).unwrap(),
                OpWidth::Word => {
                    cpu.bus.load(addr, BusTxSize::Bits32).unwrap() as u32 as i32 as i64 as u64
                }
                _ => panic!(),
            };

            match instr.op_width {
                OpWidth::DoubleWord => {
                    let new = original & cpu.regs[rs_2];
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits64).unwrap();
                }
                OpWidth::Word => {
                    let new = (original as u32 & cpu.regs[rs_2] as u32) as i32 as i64 as u64;
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits32).unwrap();
                }
                _ => panic!(),
            };
        }
        InstrOp::AtomicOr => {
            let InstrFlow::RegRegMem2RegMem { rs_1, rs_2, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let original = match instr.op_width {
                OpWidth::DoubleWord => cpu.bus.load(addr, BusTxSize::Bits64).unwrap(),
                OpWidth::Word => {
                    cpu.bus.load(addr, BusTxSize::Bits32).unwrap() as u32 as i32 as i64 as u64
                }
                _ => panic!(),
            };

            match instr.op_width {
                OpWidth::DoubleWord => {
                    let new = original | cpu.regs[rs_2];
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits64).unwrap();
                }
                OpWidth::Word => {
                    let new = (original as u32 | cpu.regs[rs_2] as u32) as i32 as i64 as u64;
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits32).unwrap();
                }
                _ => panic!(),
            };
        }
        InstrOp::AtomicMin => {
            let InstrFlow::RegRegMem2RegMem { rs_1, rs_2, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let original = match instr.op_width {
                OpWidth::DoubleWord | OpWidth::DoubleWordUnsigned => {
                    cpu.bus.load(addr, BusTxSize::Bits64).unwrap()
                }
                OpWidth::Word => {
                    cpu.bus.load(addr, BusTxSize::Bits32).unwrap() as u32 as i32 as i64 as u64
                }
                OpWidth::WordUnsigned => cpu.bus.load(addr, BusTxSize::Bits32).unwrap(),
                _ => panic!(),
            };

            match instr.op_width {
                OpWidth::DoubleWord => {
                    let new = (original as i64).min(cpu.regs[rs_2] as i64);
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new as u64, BusTxSize::Bits64).unwrap();
                }
                OpWidth::DoubleWordUnsigned => {
                    let new = original.min(cpu.regs[rs_2]);
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits64).unwrap();
                }
                OpWidth::Word => {
                    let new = (original as u32 as i32).min(cpu.regs[rs_2] as u32 as i32);
                    cpu.regs[rd] = original;
                    cpu.bus
                        .store(addr, new as u32 as u64, BusTxSize::Bits32)
                        .unwrap();
                }
                OpWidth::WordUnsigned => {
                    let new = (original as u32).min(cpu.regs[rs_2] as u32);
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new as u64, BusTxSize::Bits32).unwrap();
                }
                _ => panic!(),
            };
        }
        InstrOp::AtomicMax => {
            let InstrFlow::RegRegMem2RegMem { rs_1, rs_2, rd } = instr.flow else {
                panic!()
            };

            let addr = match translate_addr(cpu, cpu.regs[rs_1], AccessType::Load) {
                Ok(addr) => addr,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.regs[rs_1]));
                    return;
                }
            };

            let original = match instr.op_width {
                OpWidth::DoubleWord | OpWidth::DoubleWordUnsigned => {
                    cpu.bus.load(addr, BusTxSize::Bits64).unwrap()
                }
                OpWidth::Word => {
                    cpu.bus.load(addr, BusTxSize::Bits32).unwrap() as u32 as i32 as i64 as u64
                }
                OpWidth::WordUnsigned => cpu.bus.load(addr, BusTxSize::Bits32).unwrap(),
                _ => panic!(),
            };

            match instr.op_width {
                OpWidth::DoubleWord => {
                    let new = (original as i64).max(cpu.regs[rs_2] as i64);
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new as u64, BusTxSize::Bits64).unwrap();
                }
                OpWidth::DoubleWordUnsigned => {
                    let new = original.max(cpu.regs[rs_2]);
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new, BusTxSize::Bits64).unwrap();
                }
                OpWidth::Word => {
                    let new = (original as u32 as i32).max(cpu.regs[rs_2] as u32 as i32);
                    cpu.regs[rd] = original;
                    cpu.bus
                        .store(addr, new as u32 as u64, BusTxSize::Bits32)
                        .unwrap();
                }
                OpWidth::WordUnsigned => {
                    let new = (original as u32).max(cpu.regs[rs_2] as u32);
                    cpu.regs[rd] = original;
                    cpu.bus.store(addr, new as u64, BusTxSize::Bits32).unwrap();
                }
                _ => panic!(),
            };
        }
    }
}
