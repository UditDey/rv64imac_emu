//! Minimal PLIC: 1 interrupt source (ID = 1)
//!   • two contexts
//!   • 3‑bit priorities (1–7) and per‑context threshold
//!   • full atomic *claim/complete* handshake with in‑service tracking
//!
//! Memory map follows the SiFive default @ 0x0C00_0000.
//! Only 32‑bit accesses are accepted.

use crate::{bus::BusTxSize, csr::Csr};
use std::ops::Range;

/// Bit mask for source‑1 (source‑ID == bit index)
const SRC1_MASK: u32 = 1 << 1;

pub struct Plic {
    /// Priority of source‑1 (0–7, but 0 means "never forward")
    prio_src1: u8,
    /// Pending edge detected
    pending: bool,
    /// Interrupt is currently in‑service (claimed, not yet completed)
    in_service: bool,
    /// Enable per context
    enable: [bool; 2],
    /// Threshold per context (0–7)
    threshold: [u8; 2],
}

// ───── Memory‑map (SiFive default base: 0x0C00_0000) ─────
impl Plic {
    const BASE: u64 = 0x0C00_0000;
    const PRIO1: u64 = Self::BASE + 0x0004;
    const PENDING: u64 = Self::BASE + 0x1000;

    const ENABLE_CTX0: u64 = Self::BASE + 0x2000;
    const ENABLE_CTX1: u64 = Self::BASE + 0x2080;

    const CTX0_BASE: u64 = Self::BASE + 0x20_0000;
    const CTX1_BASE: u64 = Self::BASE + 0x20_1000;
    const THRESH: u64 = 0x0;
    const CLAIM: u64 = 0x4;

    /// Register span exposed to the bus (exclusive upper bound)
    pub const ADDR_RANGE: Range<u64> = Self::BASE..(Self::CTX1_BASE + 0x1008);

    pub fn new() -> Self {
        Self {
            prio_src1: 0,
            pending: false,
            in_service: false,
            enable: [false; 2],
            threshold: [0; 2],
        }
    }

    // ───── Device‑side helpers ─────
    #[inline]
    pub fn raise_irq(&mut self) {
        self.pending = true; // level‑ or edge‑sensitive devices both work
    }

    #[inline]
    pub fn clear_irq(&mut self) {
        self.pending = false;
    }

    // ───── Bus interface ─────
    pub fn load(&mut self, addr: u64, size: BusTxSize) -> u64 {
        assert!(size == BusTxSize::Bits32);
        match addr {
            // Priority registers
            Self::PRIO1 => self.prio_src1 as u64,

            // Pending bitmap – bit1 corresponds to source‑ID 1
            Self::PENDING => {
                if self.pending {
                    SRC1_MASK as u64
                } else {
                    0
                }
            }

            // Enable bitmaps – bit1 controls source‑1
            Self::ENABLE_CTX0 => {
                if self.enable[0] {
                    SRC1_MASK as u64
                } else {
                    0
                }
            }
            Self::ENABLE_CTX1 => {
                if self.enable[1] {
                    SRC1_MASK as u64
                } else {
                    0
                }
            }

            // Per‑context threshold
            a if a == Self::CTX0_BASE + Self::THRESH => self.threshold[0] as u64,
            a if a == Self::CTX1_BASE + Self::THRESH => self.threshold[1] as u64,

            // *Atomic* claim read
            a if a == Self::CTX0_BASE + Self::CLAIM => self.claim(0) as u64,
            a if a == Self::CTX1_BASE + Self::CLAIM => self.claim(1) as u64,

            _ => panic!("PLIC read @ {addr:#x}"),
        }
    }

    pub fn store(&mut self, addr: u64, val: u64, size: BusTxSize) {
        assert!(size == BusTxSize::Bits32);
        let v = val as u32;
        match addr {
            // Priority – keep 3 LSBs
            Self::PRIO1 => self.prio_src1 = (v & 0x7) as u8,

            // Enables – honour bit1 only
            Self::ENABLE_CTX0 => self.enable[0] = (v & SRC1_MASK) != 0,
            Self::ENABLE_CTX1 => self.enable[1] = (v & SRC1_MASK) != 0,

            // Thresholds – keep 3 LSBs
            a if a == Self::CTX0_BASE + Self::THRESH => self.threshold[0] = (v & 0x7) as u8,
            a if a == Self::CTX1_BASE + Self::THRESH => self.threshold[1] = (v & 0x7) as u8,

            // Complete handshake – SW must write back the claimed ID (1) or 0
            a if a == Self::CTX0_BASE + Self::CLAIM || a == Self::CTX1_BASE + Self::CLAIM => {
                debug_assert!(v == 0 || v == 1, "bad PLIC complete value: {v}");
                if v == 1 {
                    self.complete();
                }
            }

            // Pending is read‑only – silently ignore writes
            Self::PENDING => {}

            _ => panic!("PLIC write @ {addr:#x}"),
        }
    }

    // ───── Internal helpers ─────
    fn claim(&mut self, ctx: usize) -> u32 {
        // Only one context may hold the interrupt at a time.
        if self.in_service {
            return 0;
        }

        let eligible = self.pending && self.enable[ctx] && (self.prio_src1 > self.threshold[ctx]);

        if eligible {
            self.pending = false; // atomic clear
            self.in_service = true; // mark as in‑service
            1
        } else {
            0
        }
    }

    fn complete(&mut self) {
        self.in_service = false;
    }

    /// Drive MEIP/SEIP bits in `mip` based on current state
    pub fn poll_interrupts(&self, csr: &mut Csr) {
        let m_pending = (self.pending || self.in_service)
            && self.enable[0]
            && (self.prio_src1 > self.threshold[0]);
        let s_pending = (self.pending || self.in_service)
            && self.enable[1]
            && (self.prio_src1 > self.threshold[1]);

        csr.update_mip(|mip| {
            mip.set_meip(m_pending);
            mip.set_seip(s_pending);
        });
    }
}
