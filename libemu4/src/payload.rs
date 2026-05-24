use crate::{Cpu, devices::ram::Ram};

const OPEN_SBI_PAYLOAD: &[u8] =
    include_bytes!("../../third_party/buildroot/output/images/fw_payload.bin");
const INITRD: &[u8] = include_bytes!("../../third_party/buildroot/output/images/rootfs.cpio");
const DEVICE_TREE: &[u8] = include_bytes!("../../device_tree.dtb");

pub const BOOT_VECTOR: u64 = Ram::BASE_ADDR;
pub const DEVICE_TREE_ADDR: u64 = Ram::BASE_ADDR + OPEN_SBI_PAYLOAD.len() as u64;

const INITRD_START_MAGIC: u32 = 0xDEADBEEF;
const INITRD_END_MAGIC: u32 = 0xFEE1DEAD;
const MEMORY_SIZE_MAGIC: u32 = 0xBADC0DE;

pub fn install(cpu: &mut Cpu) {
    let ram = &mut cpu.bus.ram.0;

    // Install OpenSBI payload + device tree
    ram.iter_mut()
        .zip(OPEN_SBI_PAYLOAD.iter().chain(DEVICE_TREE))
        .for_each(|(ram, payload)| *ram = *payload);

    // Install initrd
    let initrd_offset = ram.len() - INITRD.len();

    ram[initrd_offset..]
        .iter_mut()
        .zip(INITRD)
        .for_each(|(ram, payload)| *ram = *payload);

    // Patch device tree
    let mem_size = ram.len();
    let dt = &mut ram[OPEN_SBI_PAYLOAD.len()..(OPEN_SBI_PAYLOAD.len() + DEVICE_TREE.len())];

    patch(
        dt,
        INITRD_START_MAGIC,
        (Ram::BASE_ADDR + initrd_offset as u64).try_into().unwrap(),
    );
    patch(
        dt,
        INITRD_END_MAGIC,
        (Ram::BASE_ADDR + initrd_offset as u64 + INITRD.len() as u64)
            .try_into()
            .unwrap(),
    );
    patch(dt, MEMORY_SIZE_MAGIC, mem_size.try_into().unwrap());

    /*panic!(
        "initrd start = 0x{:X}, end = 0x{:X}\nmemory size = 0x{:x}",
        Ram::ADDR_RANGE.end - INITRD.len() as u64,
        Ram::ADDR_RANGE.end,
        ram.len()
    );*/

    // Point OpenSBI to the device tree
    cpu.regs[11] = DEVICE_TREE_ADDR;

    // Set boot vector
    cpu.pc = BOOT_VECTOR;
}

/// LLM WRITTEN !!!
pub fn patch(buf: &mut [u8], magic: u32, replacement: u32) {
    let magic_bytes = magic.to_be_bytes();
    let replacement_bytes = replacement.to_be_bytes();

    let mut found_offset: Option<usize> = None;
    let mut idx = 0; // current position in pattern

    for (i, &b) in buf.iter().enumerate() {
        if b == magic_bytes[idx] {
            idx += 1;
            // full match?
            if idx == magic_bytes.len() {
                let start = i + 1 - magic_bytes.len();
                if found_offset.is_some() {
                    panic!("multiple magic values (0x{magic:X}) found");
                }
                found_offset = Some(start);
                idx = 0; // reset for possible second match
            }
        } else {
            // fallback: if current byte matches first byte of pattern,
            // restart at 1, else restart at 0.
            idx = if b == magic_bytes[0] { 1 } else { 0 };
        }
    }

    match found_offset {
        Some(off) => buf[off..off + 4].copy_from_slice(&replacement_bytes),
        None => panic!("magic value (0x{magic:X}) not found"),
    }
}
