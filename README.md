# Emu4 - RV64IMAC Emulator in Rust

This is a complete application-class single-core RISC-V emulator running OpenSBI + Linux

- Complete RV64IMAC instruction coverage
- [Buildroot](https://github.com/buildroot/buildroot) used to compile the OS components
- Core RISC-V peripherals implemented (CLINT and PLIC)
- Additional peripherals (UART and generic Linux Syscon)
- Cosimulation shim using [Spike](https://github.com/riscv-software-src/riscv-isa-sim) used to debug issues and maintain parity

### Overview
- [`libemu4`](libemu4): The core emulation library
  - [`lib.rs`](libemu4/src/lib.rs): Contains the top-level CPU struct
  - [`csr.rs`](libemu4/src/csr.rs): The CSR register file with CPU mode and read/write access logic
  - [`mmu.rs`](libemu4/src/mmu.rs): `Sv39` compliant MMU
  - [`trap.rs`](libemu4/src/trap.rs): Interrupt and exception handling logic
  - [`decode`](libemu4/src/decode): Instruction decoder module including compressed 16-bit instructions
  - [`exec.rs`](libemu4/src/exec.rs): Core instruction execution logic
  - [`devices`](libemu4/src/devices): Peripheral devices including PLIC, CLINT, 8250/16550 compatible UART, RAM and Syscon
- [`libcosim`](libcosim): Per-cycle Spike cosimulation library
- [`emu4_gold`](emu4_gold): Top-level RV64IMAC simulator binary with Parquet based instruction tracing
