use bitfield_macro::bitfield;

use super::{Instr, InstrFlow, InstrOp, OpWidth, sign_extend};

fn op(instr: u16) -> u16 {
    instr & 0b11
}

fn funct_3(instr: u16) -> u16 {
    instr >> 13
}

pub fn decode(instr: u16) -> Option<Instr> {
    let instr_raw = instr;

    let (op, flow, op_width) = match op(instr) {
        // Quadrant 0
        0b00 => match funct_3(instr) {
            // c.lw
            0b010 => {
                bitfield! {
                    struct Instr<u16> {
                        rd: 2..=4,
                        offset_6: 5,
                        offset_2: 6,
                        rs_1: 7..=9,
                        offset_5_3: 10..=12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_1 = instr.rs_1() as usize + 8;
                let rd = instr.rd() as usize + 8;

                let imm = ((instr.offset_6() as u64) << 6)
                    | ((instr.offset_5_3() as u64) << 3)
                    | ((instr.offset_2() as u64) << 2);

                (
                    InstrOp::Load,
                    InstrFlow::RegImmMem2Reg { rs_1, imm, rd },
                    OpWidth::Word,
                )
            }

            // c.ld
            0b011 => {
                bitfield! {
                    struct Instr<u16> {
                        rd: 2..=4,
                        imm_7_6: 5..=6,
                        rs_1: 7..=9,
                        imm_5_3: 10..=12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_1 = instr.rs_1() as usize + 8;
                let rd = instr.rd() as usize + 8;
                let imm = ((instr.imm_7_6() as u64) << 6) | ((instr.imm_5_3() as u64) << 3);

                (
                    InstrOp::Load,
                    InstrFlow::RegImmMem2Reg { rs_1, imm, rd },
                    OpWidth::DoubleWord,
                )
            }

            // c.sw
            0b110 => {
                bitfield! {
                    struct Instr<u16> {
                        rs_2: 2..=4,
                        offset_6: 5,
                        offset_2: 6,
                        rs_1: 7..=9,
                        offset_5_3: 10..=12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_1 = instr.rs_1() as usize + 8;
                let rs_2 = instr.rs_2() as usize + 8;
                let imm = ((instr.offset_6() as u64) << 6)
                    | ((instr.offset_5_3() as u64) << 3)
                    | ((instr.offset_2() as u64) << 2);

                (
                    InstrOp::Store,
                    InstrFlow::RegRegImm2Mem { rs_1, rs_2, imm },
                    OpWidth::Word,
                )
            }

            // c.addi4spn
            0b000 => {
                bitfield! {
                    struct Instr<u16> {
                        rd: 2..=4,
                        zimm_3: 5,
                        zimm_2: 6,
                        zimm_9_6: 7..=10,
                        zimm_5_4: 11..=12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize + 8;
                let imm = ((instr.zimm_9_6() as u64) << 6)
                    | ((instr.zimm_5_4() as u64) << 4)
                    | ((instr.zimm_3() as u64) << 3)
                    | ((instr.zimm_2() as u64) << 2);

                (
                    InstrOp::Add,
                    InstrFlow::RegImm2Reg { rs_1: 2, imm, rd },
                    OpWidth::DoubleWord,
                )
            }

            // c.sd
            0b111 => {
                bitfield! {
                    struct Instr<u16> {
                        rs_2: 2..=4,
                        offset_7_6: 5..=6,
                        rs_1: 7..=9,
                        offset_5_3: 10..=12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_1 = instr.rs_1() as usize + 8;
                let rs_2 = instr.rs_2() as usize + 8;
                let imm = ((instr.offset_7_6() as u64) << 6) | ((instr.offset_5_3() as u64) << 3);

                (
                    InstrOp::Store,
                    InstrFlow::RegRegImm2Mem { rs_1, rs_2, imm },
                    OpWidth::DoubleWord,
                )
            }

            _ => return None,
        },

        // Quadrant 1
        0b01 => match funct_3(instr) {
            // c.j
            0b101 => {
                bitfield! {
                    struct Instr<u16> {
                        offset_5: 2,
                        offset_3_1: 3..=5,
                        offset_7: 6,
                        offset_6: 7,
                        offset_10: 8,
                        offset_9_8: 9..=10,
                        offset_4: 11,
                        offset_11: 12
                    }
                }

                let instr = Instr::from_bits(instr);

                let imm = ((instr.offset_11() as u64) << 11)
                    | ((instr.offset_10() as u64) << 10)
                    | ((instr.offset_9_8() as u64) << 8)
                    | ((instr.offset_7() as u64) << 7)
                    | ((instr.offset_6() as u64) << 6)
                    | ((instr.offset_5() as u64) << 5)
                    | ((instr.offset_4() as u64) << 4)
                    | ((instr.offset_3_1() as u64) << 1);

                (
                    InstrOp::JumpAndLink,
                    InstrFlow::ImmPC2RegPC {
                        imm: sign_extend(imm, 11),
                        rd: 0,
                    },
                    OpWidth::None,
                )
            }

            // c.addiw
            0b001 => {
                bitfield! {
                    struct Instr<u16> {
                        imm_4_0: 2..=6,
                        rd: 7..=11,
                        imm_5: 12
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize;
                let imm = ((instr.imm_5() as u64) << 5) | (instr.imm_4_0() as u64);
                let imm = sign_extend(imm, 5);

                (
                    InstrOp::Add,
                    InstrFlow::RegImm2Reg { rs_1: rd, imm, rd },
                    OpWidth::Word,
                )
            }

            // c.beqz
            0b110 => {
                bitfield! {
                    struct Instr<u16> {
                        offset_5: 2,
                        offset_2_1: 3..=4,
                        offset_7_6: 5..=6,
                        rs_1: 7..=9,
                        offset_4_3: 10..=11,
                        offset_8: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_1 = instr.rs_1() as usize + 8;
                let imm = ((instr.offset_8() as u64) << 8)
                    | ((instr.offset_7_6() as u64) << 6)
                    | ((instr.offset_5() as u64) << 5)
                    | ((instr.offset_4_3() as u64) << 3)
                    | ((instr.offset_2_1() as u64) << 1);

                (
                    InstrOp::BranchIfEquals,
                    InstrFlow::RegRegImm2PC {
                        rs_1,
                        rs_2: 0,
                        imm: sign_extend(imm, 8),
                    },
                    OpWidth::None,
                )
            }

            // c.bnez
            0b111 => {
                bitfield! {
                    struct Instr<u16> {
                        offset_5: 2,
                        offset_2_1: 3..=4,
                        offset_7_6: 5..=6,
                        rs_1: 7..=9,
                        offset_4_3: 10..=11,
                        offset_8: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_1 = instr.rs_1() as usize + 8;
                let imm = ((instr.offset_8() as u64) << 8)
                    | ((instr.offset_7_6() as u64) << 6)
                    | ((instr.offset_5() as u64) << 5)
                    | ((instr.offset_4_3() as u64) << 3)
                    | ((instr.offset_2_1() as u64) << 1);

                (
                    InstrOp::BranchIfNotEquals,
                    InstrFlow::RegRegImm2PC {
                        rs_1,
                        rs_2: 0,
                        imm: sign_extend(imm, 8),
                    },
                    OpWidth::None,
                )
            }

            // c.li
            0b010 => {
                bitfield! {
                    struct Instr<u16> {
                        imm_4_0: 2..=6,
                        rd: 7..=11,
                        imm_5: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize;
                let imm = ((instr.imm_5() as u64) << 5) | (instr.imm_4_0() as u64);

                (
                    InstrOp::Add,
                    InstrFlow::RegImm2Reg {
                        rs_1: 0,
                        imm: sign_extend(imm, 5),
                        rd,
                    },
                    OpWidth::DoubleWord,
                )
            }

            // c.lui / c.addi16sp
            0b011 => {
                bitfield! {
                    struct Instr<u16> {
                        imm_4_0: 2..=6,
                        rd: 7..=11,
                        imm_5: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize;
                let imm = ((instr.imm_5() as u64) << 5) | (instr.imm_4_0() as u64);

                match rd {
                    2 => addi16sp_decode(instr_raw),
                    0 => return None,
                    rd => (
                        InstrOp::LoadUpperImm,
                        InstrFlow::Imm2Reg {
                            imm: sign_extend(imm << 12, 17),
                            rd,
                        },
                        OpWidth::None,
                    ),
                }
            }

            // c.addi
            0b000 => {
                bitfield! {
                    struct Instr<u16> {
                        imm_4_0: 2..=6,
                        rd: 7..=11,
                        imm_5: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize;
                let imm = ((instr.imm_5() as u64) << 5) | (instr.imm_4_0() as u64);

                (
                    InstrOp::Add,
                    InstrFlow::RegImm2Reg {
                        rs_1: rd,
                        imm: sign_extend(imm, 5),
                        rd,
                    },
                    OpWidth::DoubleWord,
                )
            }

            // c.srli / c.srai / c.andi /
            0b100 => {
                bitfield! {
                    struct Instr<u16> {
                        shamt_4_0: 2..=6,
                        rd: 7..=9,
                        funct_2: 10..=11,
                        shamt_5: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize + 8;
                let imm = ((instr.shamt_5() as u64) << 5) | (instr.shamt_4_0() as u64);
                let imm = sign_extend(imm, 5);

                match instr.funct_2() {
                    0b00 => (
                        InstrOp::ShiftRightLogical,
                        InstrFlow::RegImm2Reg { rs_1: rd, imm, rd },
                        OpWidth::DoubleWord,
                    ),
                    0b01 => (
                        InstrOp::ShiftRightArithmetic,
                        InstrFlow::RegImm2Reg { rs_1: rd, imm, rd },
                        OpWidth::DoubleWord,
                    ),
                    0b10 => (
                        InstrOp::And,
                        InstrFlow::RegImm2Reg { rs_1: rd, imm, rd },
                        OpWidth::None,
                    ),
                    0b11 => return reg_reg_ops_decode(instr_raw),
                    _ => unreachable!(),
                }
            }

            _ => return None,
        },

        // Quadrant 2
        0b10 => match funct_3(instr) {
            // c.lwsp
            0b010 => {
                bitfield! {
                    struct Instr<u16> {
                        offset_7_6: 2..=3,
                        offset_4_2: 4..=6,
                        rd: 7..=11,
                        offset_5: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize;
                let imm = ((instr.offset_7_6() as u64) << 6)
                    | ((instr.offset_5() as u64) << 5)
                    | ((instr.offset_4_2() as u64) << 2);

                (
                    InstrOp::Load,
                    InstrFlow::RegImmMem2Reg { rs_1: 2, imm, rd },
                    OpWidth::Word,
                )
            }

            // c.swsp
            0b110 => {
                bitfield! {
                    struct Instr<u16> {
                        rs_2: 2..=6
                        offset_7_6: 7..=8,
                        offset_5_2: 9..=12
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_2 = instr.rs_2() as usize;
                let imm = ((instr.offset_7_6() as u64) << 6) | ((instr.offset_5_2() as u64) << 2);

                (
                    InstrOp::Store,
                    InstrFlow::RegRegImm2Mem { rs_1: 2, rs_2, imm },
                    OpWidth::Word,
                )
            }

            // c.sdsp
            0b111 => {
                bitfield! {
                    struct Instr<u16> {
                        rs_2: 2..=6
                        offset_8_6: 7..=9,
                        offset_5_3: 10..=12
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_2 = instr.rs_2() as usize;
                let imm = ((instr.offset_8_6() as u64) << 6) | ((instr.offset_5_3() as u64) << 3);

                (
                    InstrOp::Store,
                    InstrFlow::RegRegImm2Mem { rs_1: 2, rs_2, imm },
                    OpWidth::DoubleWord,
                )
            }

            // c.ldsp
            0b011 => {
                bitfield! {
                    struct Instr<u16> {
                        offset_8_6: 2..=4,
                        offset_4_3: 5..=6,
                        rd: 7..=11,
                        offset_5: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize;
                let imm = ((instr.offset_8_6() as u64) << 6)
                    | ((instr.offset_5() as u64) << 5)
                    | ((instr.offset_4_3() as u64) << 3);

                (
                    InstrOp::Load,
                    InstrFlow::RegImmMem2Reg { rs_1: 2, imm, rd },
                    OpWidth::DoubleWord,
                )
            }

            // c.jr / c.jalr
            0b100 => {
                bitfield! {
                    struct Instr<u16> {
                        rs_2: 2..=6,
                        rs_1: 7..=11,
                        funct_4: 12..=15
                    }
                }

                let instr = Instr::from_bits(instr);

                let rs_1 = instr.rs_1() as usize;
                let rs_2 = instr.rs_2() as usize;

                match (rs_1, rs_2) {
                    (0, 0) => (InstrOp::EnvBreak, InstrFlow::None, OpWidth::None),
                    (rs_1, 0) => match instr.funct_4() {
                        0b1000 => (
                            InstrOp::JumpAndLinkReg,
                            InstrFlow::RegImmPC2RegPC {
                                rs_1,
                                imm: 0,
                                rd: 0,
                            },
                            OpWidth::None,
                        ),
                        0b1001 => (
                            InstrOp::JumpAndLinkReg,
                            InstrFlow::RegImmPC2RegPC {
                                rs_1,
                                imm: 0,
                                rd: 1,
                            },
                            OpWidth::None,
                        ),
                        _ => return None,
                    },
                    (rs_1, rs_2) => match instr.funct_4() {
                        0b1000 => (
                            InstrOp::Add,
                            InstrFlow::RegReg2Reg {
                                rs_1: 0,
                                rs_2,
                                rd: rs_1,
                            },
                            OpWidth::DoubleWord,
                        ),
                        0b1001 => (
                            InstrOp::Add,
                            InstrFlow::RegReg2Reg {
                                rs_1,
                                rs_2,
                                rd: rs_1,
                            },
                            OpWidth::DoubleWord,
                        ),
                        _ => return None,
                    },
                }
            }

            // c.slli
            0b000 => {
                bitfield! {
                    struct Instr<u16> {
                        shamt_4_0: 2..=6,
                        rd: 7..=11,
                        shamt_5: 12,
                    }
                }

                let instr = Instr::from_bits(instr);

                let rd = instr.rd() as usize;
                let imm = ((instr.shamt_5() as u64) << 5) | (instr.shamt_4_0() as u64);

                (
                    InstrOp::ShiftLeft,
                    InstrFlow::RegImm2Reg { rs_1: rd, imm, rd },
                    OpWidth::DoubleWord,
                )
            }

            _ => return None,
        },

        _ => return None,
    };

    Some(Instr {
        raw: instr_raw as u32,
        op,
        flow,
        op_width,
        instr_size: 2,
    })
}

fn addi16sp_decode(instr: u16) -> (InstrOp, InstrFlow, OpWidth) {
    bitfield! {
        struct Instr<u16> {
            imm_5: 2,
            imm_8_7: 3..=4,
            imm_6: 5,
            imm_4: 6,
            imm_9: 12
        }
    }

    let instr = Instr::from_bits(instr);

    let imm = ((instr.imm_9() as u64) << 9)
        | ((instr.imm_8_7() as u64) << 7)
        | ((instr.imm_6() as u64) << 6)
        | ((instr.imm_5() as u64) << 5)
        | ((instr.imm_4() as u64) << 4);

    (
        InstrOp::Add,
        InstrFlow::RegImm2Reg {
            rs_1: 2,
            imm: sign_extend(imm, 9),
            rd: 2,
        },
        OpWidth::DoubleWord,
    )
}

fn reg_reg_ops_decode(instr: u16) -> Option<Instr> {
    let instr_raw = instr;

    bitfield! {
        struct Instr<u16> {
            rs_2: 2..=4,
            funct_2: 5..=6,
            rd: 7..=9,
            funct_6: 10..=15
        }
    }

    let instr = Instr::from_bits(instr);

    let rs_2 = instr.rs_2() as usize + 8;
    let rd = instr.rd() as usize + 8;

    let (op, op_width) = match instr.funct_2() {
        0b00 => match instr.funct_6() {
            0b100011 => (InstrOp::Subtract, OpWidth::DoubleWord),
            0b100111 => (InstrOp::Subtract, OpWidth::Word),
            _ => panic!(),
        },
        0b10 => (InstrOp::Or, OpWidth::None),
        0b01 => match instr.funct_6() {
            0b100111 => (InstrOp::Add, OpWidth::Word),
            0b100011 => (InstrOp::Xor, OpWidth::None),
            _ => return None,
        },
        0b11 => (InstrOp::And, OpWidth::None),
        _ => unreachable!(),
    };

    Some(super::Instr {
        raw: instr_raw as u32,
        op,
        flow: InstrFlow::RegReg2Reg { rs_1: rd, rs_2, rd },
        op_width,
        instr_size: 2,
    })
}
