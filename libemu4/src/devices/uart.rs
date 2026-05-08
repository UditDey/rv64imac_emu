//! LLM WRITTEN !!!
//!
//! Very-small 8250/16550 model
//! – 8-bit register spacing (reg-shift = 0)
//! – one PLIC source `UART_IRQ_ID`
//! – RX 16-byte FIFO, immediate TX “send”
//! – implements THR/RBR, IER, IIR/FCR, LCR, LSR, MCR, MSR, DLL/DLH

use std::{
    collections::VecDeque,
    io::{Read, Write},
    ops::Range,
    sync::mpsc::Receiver,
};

use crate::{bus::BusTxSize, devices::plic::Plic};

const UART_BASE: u64 = 0x1000_0000;
const UART_SIZE: u64 = 0x100;

const _UART_IRQ_ID: usize = 1; // matches <&plic0 1>

pub struct Uart {
    /* registers */
    ier: u8,
    lcr: u8,
    mcr: u8,
    dll: u8,
    dlh: u8,
    lsr: u8, // dynamic bits regenerated
    msr: u8,
    /* fifos */
    rx: VecDeque<u8>,
    tx_holding: Option<u8>,
    /* host stdin */
    stdin_rx: Receiver<u8>,
    /* irq line */
    irq_state: bool,
    silent: bool,
}

impl Uart {
    pub const ADDR_RANGE: Range<u64> = UART_BASE..UART_BASE + UART_SIZE;

    /* ---- helpers for bit masks ---- */
    const IER_RX: u8 = 1 << 0;
    const IER_TX: u8 = 1 << 1;

    const LSR_DR: u8 = 1 << 0;
    const LSR_OE: u8 = 1 << 1;
    const LSR_THRE: u8 = 1 << 5;
    const LSR_TEMT: u8 = 1 << 6;

    const IIR_NO_INT: u8 = 0x01;
    const IIR_TX_EMPTY: u8 = 0x02;
    const IIR_RX_AVAIL: u8 = 0x04;

    /* ---- constructor -------------------------------------------------- */
    pub fn new() -> Self {
        //crossterm::terminal::enable_raw_mode().unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut stdin = std::io::stdin();
            let mut byte = [0u8; 1];
            loop {
                if stdin.read_exact(&mut byte).is_ok() {
                    let _ = tx.send(byte[0]);
                }
            }
        });

        Self {
            ier: 0,
            lcr: 0x03, // 8-N-1, DLAB=0
            mcr: 0,
            dll: 0,
            dlh: 0,
            lsr: Self::LSR_THRE | Self::LSR_TEMT,
            msr: 0,
            rx: VecDeque::with_capacity(16),
            tx_holding: None,
            stdin_rx: rx,
            irq_state: false,
            silent: false,
        }
    }

    pub fn silent() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            ier: 0,
            lcr: 0x03, // 8-N-1, DLAB=0
            mcr: 0,
            dll: 0,
            dlh: 0,
            lsr: Self::LSR_THRE | Self::LSR_TEMT,
            msr: 0,
            rx: VecDeque::with_capacity(16),
            tx_holding: None,
            stdin_rx: rx,
            irq_state: false,
            silent: true,
        }
    }

    /* ---- public bus interface ---------------------------------------- */
    pub fn load(&mut self, addr: u64, size: BusTxSize, plic: &mut Plic) -> u64 {
        assert!(size == BusTxSize::Bits8);

        let off = addr - UART_BASE;
        let dlab = self.lcr & 0x80 != 0;
        let val = match off {
            0x0 => {
                // RBR or DLL
                if dlab {
                    self.dll as u64
                } else {
                    let b = self.rx.pop_front().unwrap_or(0);
                    if self.rx.is_empty() {
                        self.lsr &= !Self::LSR_DR;
                    }
                    b as u64
                }
            }
            0x1 => {
                if dlab {
                    self.dlh as u64
                } else {
                    self.ier as u64
                }
            }
            0x2 => self.calc_iir() as u64,
            0x3 => self.lcr as u64,
            0x4 => self.mcr as u64,
            0x5 => self.lsr as u64,
            0x6 => self.msr as u64,
            0x7 => 0, // scratch
            _ => 0,
        };
        self.update_irq(plic);
        val
    }

    pub fn store(&mut self, addr: u64, val: u64, size: BusTxSize, plic: &mut Plic) {
        assert!(size == BusTxSize::Bits8);
        let b = val as u8;
        let off = addr - UART_BASE;
        let dlab = self.lcr & 0x80 != 0;

        match off {
            0x0 => {
                // THR or DLL
                if dlab {
                    self.dll = b;
                } else {
                    self.tx_holding = Some(b);
                    self.lsr &= !Self::LSR_THRE; // busy for one cycle
                }
            }
            0x1 => {
                if dlab {
                    self.dlh = b;
                } else {
                    self.ier = b & 0x0F;
                }
            }
            0x2 => {
                // FCR write: clear fifos
                if b & 1 != 0 {
                    self.rx.clear();
                    self.lsr &= !Self::LSR_DR;
                }
            }
            0x3 => self.lcr = b,
            0x4 => self.mcr = b,
            _ => {}
        }
        self.update_irq(plic);
    }

    /* ---- tick: drive TX, poll stdin ---------------------------------- */
    pub fn tick(&mut self, plic: &mut Plic) {
        /* send TX byte, mark empty */
        if let Some(ch) = self.tx_holding.take() {
            if !self.silent {
                print!("{}", ch as char);
                std::io::stdout().flush().unwrap();
            }

            self.lsr |= Self::LSR_THRE | Self::LSR_TEMT;
        }

        /* poll host stdin */
        if let Ok(byte) = self.stdin_rx.try_recv() {
            if self.rx.len() >= 16 {
                self.lsr |= Self::LSR_OE; // overrun flag
            } else {
                self.rx.push_back(byte);
                self.lsr |= Self::LSR_DR;
            }
        }
        self.update_irq(plic);
    }

    /* ---- helpers ----------------------------------------------------- */
    fn calc_iir(&self) -> u8 {
        if self.lsr & Self::LSR_DR != 0 && self.ier & Self::IER_RX != 0 {
            Self::IIR_RX_AVAIL
        } else if self.lsr & Self::LSR_THRE != 0 && self.ier & Self::IER_TX != 0 {
            Self::IIR_TX_EMPTY
        } else {
            Self::IIR_NO_INT
        }
    }

    fn update_irq(&mut self, plic: &mut Plic) {
        let want_irq = self.calc_iir() != Self::IIR_NO_INT;

        if want_irq && !self.irq_state {
            plic.raise_irq();
            self.irq_state = true;
        } else if !want_irq && self.irq_state {
            plic.clear_irq();
            self.irq_state = false;
        }
    }
}
