//! Sv39 Software MMU – reference implementation (no TLB)
//! =====================================================
//! * ISA: RISC‑V Privileged Specification v1.12, §4.3.2 (Sv39)
//! * Scope: functional page‑table walk with precise side‑effects (A/D bits)
//! * Only critical fixes applied compared with the previous revision.

use crate::{
    CpuMode,
    bus::{Bus, BusTxSize},
    csr::Csr,
    trap::Exception,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessType {
    InstructionFetch,
    Load,
    Store,
}

#[inline(always)]
fn page_fault(access: AccessType) -> Exception {
    match access {
        AccessType::InstructionFetch => Exception::InstructionPageFault,
        AccessType::Load => Exception::LoadPageFault,
        AccessType::Store => Exception::StorePageFault,
    }
}

// ---- Architectural constants ------------------------------------------------

const PTE_SIZE: u64 = 8;
const SV39_LEVELS: usize = 3;
const SV39_MODE: u64 = 8;

// PTE bit definitions
const PTE_V: u64 = 1 << 0;
const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const PTE_U: u64 = 1 << 4;
const PTE_G: u64 = 1 << 5;
const PTE_A: u64 = 1 << 6;
const PTE_D: u64 = 1 << 7;

#[inline(always)]
fn vpn(va: u64, level: usize) -> u64 {
    (va >> (12 + 9 * level)) & 0x1FF
}

/// Sv39 canonical‑address test (spec §4.3.2, step 0)
#[inline(always)]
fn is_canonical(va: u64) -> bool {
    let sign_bit = (va >> 38) & 1;
    let high = va >> 39;
    if sign_bit == 0 {
        high == 0
    } else {
        high == ((1u64 << 25) - 1)
    }
}

#[inline(always)]
fn compose_pa(ppn: u64, va: u64, level: usize) -> u64 {
    // Extract VPN pieces from VA
    let vpn0 = (va >> 12) & 0x1FF;
    let vpn1 = (va >> 21) & 0x1FF;
    let vpn2 = (va >> 30) & 0x1FF;
    let off = va & 0xFFF;

    // Extract PPN pieces from PTE
    let ppn0 = ppn & 0x1FF;
    let ppn1 = (ppn >> 9) & 0x1FF;
    let ppn2 = (ppn >> 18) & 0x3FF_FFFF; // 26 bits

    let (new0, new1, new2) = match level {
        0 => (ppn0, ppn1, ppn2), // 4 KiB leaf
        1 => (vpn0, ppn1, ppn2), // 2 MiB leaf
        2 => (vpn0, vpn1, ppn2), // 1 GiB leaf
        _ => unreachable!(),
    };

    (new2 << 30) | (new1 << 21) | (new0 << 12) | off
}

// ---- MMU proper --------------------------------------------------------------

pub struct Mmu;

impl Mmu {
    pub fn new() -> Self {
        Self
    }

    /// Translate `va` under the current privilege & CSR state.
    /// Returns a **physical** address or an appropriate page‑fault trap.
    pub fn translate(
        &mut self,
        va: u64,
        access: AccessType,
        mut mode: CpuMode,
        csr: &Csr,
        bus: &mut Bus,
    ) -> Result<u64, Exception> {
        // MPRV remapping (spec §1.12, 1.7.6)
        if mode == CpuMode::Machine
            && csr.mstatus().mprv()
            && access != AccessType::InstructionFetch
        {
            mode = csr.mstatus().mpp().try_into().unwrap();
        }

        // Bare or machine mode → identity mapping
        let satp = csr.satp().as_bits();
        let satp_mode = satp >> 60;
        if mode == CpuMode::Machine || satp_mode == 0 {
            return Ok(va);
        }
        if satp_mode != SV39_MODE {
            //println!("emu4: MMU WRONG MODE");
            return Err(page_fault(access));
        }
        if !is_canonical(va) {
            //println!("emu4: MMU NOT CANON");
            return Err(page_fault(access));
        }

        // Walk state initialisation
        let root_ppn = satp & ((1u64 << 44) - 1);
        let mut table = root_ppn << 12;

        for level in (0..SV39_LEVELS).rev() {
            let pte_addr = table + vpn(va, level) * PTE_SIZE;
            let pte_val = bus
                .load_internal(pte_addr, BusTxSize::Bits64)
                .ok_or_else(|| page_fault(access))?;

            let valid = (pte_val & PTE_V) != 0;
            let read = (pte_val & PTE_R) != 0;
            let write = (pte_val & PTE_W) != 0;
            let exec = (pte_val & PTE_X) != 0;

            // Invalid or reserved encoding (§4.3.2, step 3)
            if !valid || (!read && write) {
                //println!("emu4: MMU INVALID BITS");
                return Err(page_fault(access));
            }

            // -----------------------------------------------------------------
            //  Leaf?  (spec step 4)
            // -----------------------------------------------------------------
            let non_leaf = (pte_val & (PTE_R | PTE_X | PTE_W)) == 0;

            if !non_leaf {
                // Super‑page alignment (§4.3.2, step 5)
                if level > 0 {
                    let ppn = (pte_val >> 10) & ((1u64 << 44) - 1);
                    let mis_mask = (1u64 << (9 * level)) - 1; // low 9·level bits
                    if ppn & mis_mask != 0 {
                        //println!("emu4: MMU SUPERPAGE FAULT");
                        return Err(page_fault(access));
                    }
                }

                let accessed = (pte_val & PTE_A) != 0;
                let dirty = (pte_val & PTE_D) != 0;
                if !accessed || (access == AccessType::Store && !dirty) {
                    return Err(page_fault(access)); // spike-compatible behaviour
                }

                // Privilege & SUM/U/S checks (spec step 6)
                let user = (pte_val & PTE_U) != 0;
                if mode == CpuMode::User && !user {
                    //println!("emu4: MMU USER PRIV FAULT");
                    return Err(page_fault(access));
                }
                if mode == CpuMode::Supervisor
                    && user
                    && !csr.mstatus().sum()
                    && access != AccessType::InstructionFetch
                {
                    //println!("emu4: MMU SUPERVISOR PRIV FAULT");
                    return Err(page_fault(access));
                }

                // Permission checks (spec step 7)
                let permitted = match access {
                    AccessType::InstructionFetch => exec,
                    AccessType::Load => read || (csr.mstatus().mxr() && exec),
                    AccessType::Store => write,
                };
                if !permitted {
                    //println!("emu4: MMU PERMISSION FAULT, access type = {access:?}");
                    return Err(page_fault(access));
                }

                // Compose physical address (spec step 8)
                let ppn = (pte_val >> 10) & ((1u64 << 44) - 1);
                let pa = compose_pa(ppn, va, level);
                return Ok(pa);
            }

            // -----------------------------------------------------------------
            //  Non‑leaf → descend (spec step 4)
            // -----------------------------------------------------------------
            let next_ppn = (pte_val >> 10) & ((1u64 << 44) - 1);
            table = next_ppn << 12;
        }

        // No matching leaf found
        //println!("emu4: MMU UNMAPPED");
        Err(page_fault(access))
    }
}
