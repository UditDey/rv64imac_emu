//mod cosim;
//mod tracer;

use std::time::Instant;

use libemu4::{
    Cpu, decode, exec,
    mmu::AccessType,
    payload,
    trap::{self, Exception},
};

//use cosim::Cosim;
//use tracer::Tracer;

fn main() {
    println!("emu4: Booting...");
    let mut cpu = Cpu::new();
    payload::install(&mut cpu);

    //let mut cosim = Cosim::new(&cpu.bus);
    let init_time = Instant::now();

    //let mut tracer = Tracer::new();

    loop {
        if cpu.bus.syscon.power_off {
            break;
        }

        let delta = Instant::now().duration_since(init_time).as_micros() as u64;

        cpu.csr.tick();
        cpu.bus.tick(&mut cpu.csr, delta);
        cpu.bus.poll_interrupts(&mut cpu.csr);

        let instr = 'fetch_exec: {
            if trap::process_interrupt(&mut cpu.csr, &mut cpu.mode, &mut cpu.pc) {
                break 'fetch_exec None;
            }

            let phys_pc = cpu.mmu.translate(
                cpu.pc,
                AccessType::InstructionFetch,
                cpu.mode,
                &cpu.csr,
                &mut cpu.bus,
            );

            let phys_pc = match phys_pc {
                Ok(pc) => pc,
                Err(exc) => {
                    cpu.exception = Some((exc, cpu.pc));
                    break 'fetch_exec None;
                }
            };

            let instr_raw = cpu.bus.fetch(phys_pc);

            let instr_raw = match instr_raw {
                Some(instr_raw) => instr_raw,
                None => {
                    cpu.exception = Some((Exception::InstructionAccessFault, cpu.pc));
                    //println!("{:X?} phys_pc = 0x{phys_pc:X}", cpu.exception);
                    break 'fetch_exec None;
                }
            };

            if instr_raw == 0 {
                cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                None
            } else {
                match decode::decode(instr_raw) {
                    Some(instr) => {
                        //tracer.trace(instr, cpu.csr.cycle(), cpu.pc, &cpu.regs);
                        exec::exec(&mut cpu, &instr, instr_raw);
                        Some(instr)
                    }
                    None => {
                        //println!("ILLEGAL 0x{instr_raw:X} @ 0x{phys_pc:X}");
                        cpu.exception = Some((Exception::IllegalInstruction, instr_raw as u64));
                        None
                    }
                }
            }
        };

        match cpu.exception.take() {
            Some((exception, exception_data)) => trap::process_exception(
                exception,
                exception_data,
                &mut cpu.csr,
                &mut cpu.mode,
                &mut cpu.pc,
            ),
            None => cpu.pc += instr.map(|instr| instr.instr_size as u64).unwrap_or(0),
        }

        cpu.regs[0] = 0;

        cpu.bus.commit_store();
        //cosim.tick(&cpu.csr, delta);
        //cosim.compare_states(cpu.pc, cpu.mode, &cpu.regs, &cpu.csr, &cpu.bus, &instr);
    }

    //tracer.finish();
}
