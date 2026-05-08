# RV64IMAC Emulator in Rust

This is a complete application-class single-core RISC-V emulator running OpenSBI + Linux

- Complete RV64IMAC instruction coverage
- [Buildroot](https://github.com/buildroot/buildroot) used to compile the OS components
- Core RISC-V peripherals implemented (CLINT and PLIC)
- Additional peripherals (UART and generic Linux Syscon)
- Cosimulation shim using [Spike](https://github.com/riscv-software-src/riscv-isa-sim) used to debug issues and maintain parity
