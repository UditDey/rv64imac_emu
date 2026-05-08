use super::{Instr, InstrFlow, InstrOp, OpWidth, sign_extend};

fn opcode(instr: u32) -> u32 {
    instr & 0b111_1111
}

fn funct_3(instr: u32) -> u32 {
    (instr >> 12) & 0b111
}

fn funct_7(instr: u32) -> u32 {
    (instr >> 25) & 0b111_1111
}

fn funct_7_atomic_type(instr: u32) -> u32 {
    (instr >> 27) & 0b1_1111
}

fn rs_1(instr: u32) -> usize {
    ((instr >> 15) & 0b1_1111) as usize
}

fn rs_2(instr: u32) -> usize {
    ((instr >> 20) & 0b1_1111) as usize
}

fn rd(instr: u32) -> usize {
    ((instr >> 7) & 0b1_1111) as usize
}

fn imm_i_type(instr: u32) -> u64 {
    sign_extend((instr >> 20) as u64, 11)
}

fn imm_u_type(instr: u32) -> u64 {
    sign_extend(
        (instr & 0b1111_1111_1111_1111_1111_0000_0000_0000) as u64,
        31,
    )
}

fn imm_j_type(instr: u32) -> u64 {
    let imm_10_1 = (instr >> 21) & 0b11_1111_1111;
    let imm_11 = (instr >> 20) & 0b1;
    let imm_19_12 = (instr >> 12) & 0b1111_1111;
    let imm_20 = (instr >> 31) & 0b1;

    let imm = (imm_20 << 20) | (imm_19_12 << 12) | (imm_11 << 11) | (imm_10_1 << 1);
    sign_extend(imm as u64, 20)
}

fn imm_b_type(instr: u32) -> u64 {
    let imm_4_1 = (instr >> 8) & 0b1111;
    let imm_10_5 = (instr >> 25) & 0b11_1111;
    let imm_11 = (instr >> 7) & 0b1;
    let imm_12 = (instr >> 31) & 0b1;

    let imm = (imm_12 << 12) | (imm_11 << 11) | (imm_10_5 << 5) | (imm_4_1 << 1);
    sign_extend(imm as u64, 12)
}

fn imm_s_type(instr: u32) -> u64 {
    let imm_4_0 = (instr >> 7) & 0b1_1111;
    let imm_11_5 = (instr >> 25) & 0b111_1111;

    let imm = (imm_11_5 << 5) | imm_4_0;
    sign_extend(imm as u64, 11)
}

pub fn decode(instr: u32) -> Option<Instr> {
    let (op, flow, op_width) = match opcode(instr) {
        0b0110111 => (
            InstrOp::LoadUpperImm,
            InstrFlow::Imm2Reg {
                imm: imm_u_type(instr),
                rd: rd(instr),
            },
            OpWidth::None,
        ),
        0b0010111 => (
            InstrOp::AddUpperImmAndPC,
            InstrFlow::ImmPC2Reg {
                imm: imm_u_type(instr),
                rd: rd(instr),
            },
            OpWidth::None,
        ),

        0b1101111 => (
            InstrOp::JumpAndLink,
            InstrFlow::ImmPC2RegPC {
                imm: imm_j_type(instr),
                rd: rd(instr),
            },
            OpWidth::None,
        ),
        0b1100111 => (
            InstrOp::JumpAndLinkReg,
            InstrFlow::RegImmPC2RegPC {
                rs_1: rs_1(instr),
                imm: imm_i_type(instr),
                rd: rd(instr),
            },
            OpWidth::None,
        ),

        0b1100011 => {
            let (op, width) = match funct_3(instr) {
                0b000 => (InstrOp::BranchIfEquals, OpWidth::DoubleWord),
                0b001 => (InstrOp::BranchIfNotEquals, OpWidth::DoubleWord),
                0b100 => (InstrOp::BranchIfLesserThan, OpWidth::DoubleWord),
                0b101 => (InstrOp::BranchIfGreaterThanOrEquals, OpWidth::DoubleWord),
                0b110 => (InstrOp::BranchIfLesserThan, OpWidth::DoubleWordUnsigned),
                0b111 => (
                    InstrOp::BranchIfGreaterThanOrEquals,
                    OpWidth::DoubleWordUnsigned,
                ),
                _ => return None,
            };

            (
                op,
                InstrFlow::RegRegImm2PC {
                    rs_1: rs_1(instr),
                    rs_2: rs_2(instr),
                    imm: imm_b_type(instr),
                },
                width,
            )
        }

        0b0000011 => {
            let op_width = match funct_3(instr) {
                0b000 => OpWidth::Byte,
                0b001 => OpWidth::HalfWord,
                0b010 => OpWidth::Word,
                0b011 => OpWidth::DoubleWord,
                0b100 => OpWidth::ByteUnsigned,
                0b101 => OpWidth::HalfWordUnsigned,
                0b110 => OpWidth::WordUnsigned,
                _ => return None,
            };

            (
                InstrOp::Load,
                InstrFlow::RegImmMem2Reg {
                    rs_1: rs_1(instr),
                    imm: imm_i_type(instr),
                    rd: rd(instr),
                },
                op_width,
            )
        }

        0b0100011 => {
            let op_width = match funct_3(instr) {
                0b000 => OpWidth::Byte,
                0b001 => OpWidth::HalfWord,
                0b010 => OpWidth::Word,
                0b011 => OpWidth::DoubleWord,
                _ => return None,
            };

            (
                InstrOp::Store,
                InstrFlow::RegRegImm2Mem {
                    rs_1: rs_1(instr),
                    rs_2: rs_2(instr),
                    imm: imm_s_type(instr),
                },
                op_width,
            )
        }

        0b0110011 => {
            let (op, width) = match funct_3(instr) {
                0b000 => match funct_7(instr) {
                    0b0000000 => (InstrOp::Add, OpWidth::DoubleWord),
                    0b0000001 => (InstrOp::Multiply, OpWidth::DoubleWord),
                    0b0100000 => (InstrOp::Subtract, OpWidth::DoubleWord),
                    _ => return None,
                },
                0b001 => match funct_7(instr) {
                    0b0000000 => (InstrOp::ShiftLeft, OpWidth::DoubleWord),
                    0b0000001 => (InstrOp::MultiplyHigh, OpWidth::DoubleWord),
                    _ => return None,
                },

                0b010 => match funct_7(instr) {
                    0b0000000 => (InstrOp::SetLesserThan, OpWidth::DoubleWord),
                    0b0000001 => (InstrOp::MultiplyHigh, OpWidth::DoubleWordSignedUnsigned),
                    _ => return None,
                },
                0b011 => match funct_7(instr) {
                    0b0000000 => (InstrOp::SetLesserThan, OpWidth::DoubleWordUnsigned),
                    0b0000001 => (InstrOp::MultiplyHigh, OpWidth::DoubleWordUnsigned),
                    _ => return None,
                },
                0b100 => match funct_7(instr) {
                    0b0000000 => (InstrOp::Xor, OpWidth::DoubleWord),
                    0b0000001 => (InstrOp::Divide, OpWidth::DoubleWord),
                    _ => return None,
                },
                0b101 => match funct_7(instr) {
                    0b0000000 => (InstrOp::ShiftRightLogical, OpWidth::DoubleWord),
                    0b0000001 => (InstrOp::Divide, OpWidth::DoubleWordUnsigned),
                    0b0100000 => (InstrOp::ShiftRightArithmetic, OpWidth::DoubleWord),
                    _ => return None,
                },
                0b110 => match funct_7(instr) {
                    0b0000000 => (InstrOp::Or, OpWidth::DoubleWord),
                    0b0000001 => (InstrOp::Remainder, OpWidth::DoubleWord),
                    _ => return None,
                },
                0b111 => match funct_7(instr) {
                    0b0000000 => (InstrOp::And, OpWidth::DoubleWord),
                    0b0000001 => (InstrOp::Remainder, OpWidth::DoubleWordUnsigned),
                    _ => return None,
                },
                _ => return None,
            };

            (
                op,
                InstrFlow::RegReg2Reg {
                    rs_1: rs_1(instr),
                    rs_2: rs_2(instr),
                    rd: rd(instr),
                },
                width,
            )
        }

        0b0111011 => {
            let (op, width) = match funct_7(instr) {
                0b0000000 => match funct_3(instr) {
                    0b000 => (InstrOp::Add, OpWidth::Word),
                    0b001 => (InstrOp::ShiftLeft, OpWidth::Word),
                    0b101 => (InstrOp::ShiftRightLogical, OpWidth::Word),
                    _ => return None,
                },
                0b0100000 => match funct_3(instr) {
                    0b000 => (InstrOp::Subtract, OpWidth::Word),
                    0b101 => (InstrOp::ShiftRightArithmetic, OpWidth::Word),
                    _ => return None,
                },
                0b0000001 => match funct_3(instr) {
                    0b000 => (InstrOp::Multiply, OpWidth::Word),
                    0b100 => (InstrOp::Divide, OpWidth::Word),
                    0b101 => (InstrOp::Divide, OpWidth::WordUnsigned),
                    0b110 => (InstrOp::Remainder, OpWidth::Word),
                    0b111 => (InstrOp::Remainder, OpWidth::WordUnsigned),
                    _ => return None,
                },
                _ => return None,
            };

            (
                op,
                InstrFlow::RegReg2Reg {
                    rs_1: rs_1(instr),
                    rs_2: rs_2(instr),
                    rd: rd(instr),
                },
                width,
            )
        }

        0b0010011 => {
            let (op, width) = match funct_3(instr) {
                0b000 => (InstrOp::Add, OpWidth::DoubleWord),
                0b001 => (InstrOp::ShiftLeft, OpWidth::DoubleWord),
                0b010 => (InstrOp::SetLesserThan, OpWidth::DoubleWord),
                0b011 => (InstrOp::SetLesserThan, OpWidth::DoubleWordUnsigned),
                0b100 => (InstrOp::Xor, OpWidth::DoubleWord),
                0b101 => match funct_7(instr) & !1 {
                    0b0000000 => (InstrOp::ShiftRightLogical, OpWidth::DoubleWord),
                    0b0100000 => (InstrOp::ShiftRightArithmetic, OpWidth::DoubleWord),
                    _ => return None,
                },
                0b110 => (InstrOp::Or, OpWidth::DoubleWord),
                0b111 => (InstrOp::And, OpWidth::DoubleWord),
                _ => return None,
            };

            (
                op,
                InstrFlow::RegImm2Reg {
                    rs_1: rs_1(instr),
                    imm: imm_i_type(instr),
                    rd: rd(instr),
                },
                width,
            )
        }

        0b0011011 => {
            let op = match funct_3(instr) {
                0b000 => InstrOp::Add,
                0b001 => InstrOp::ShiftLeft,
                0b101 => match funct_7(instr) {
                    0b0000000 => InstrOp::ShiftRightLogical,
                    0b0100000 => InstrOp::ShiftRightArithmetic,
                    _ => return None,
                },
                _ => return None,
            };

            (
                op,
                InstrFlow::RegImm2Reg {
                    rs_1: rs_1(instr),
                    imm: imm_i_type(instr),
                    rd: rd(instr),
                },
                OpWidth::Word,
            )
        }

        0b0001111 => (InstrOp::FenceI, InstrFlow::None, OpWidth::None),

        0b1110011 => {
            let reg_2_reg_csr = InstrFlow::Reg2RegCsr {
                rs_1: rs_1(instr),
                rd: rd(instr),
                csr: imm_i_type(instr),
            };

            let imm_2_reg_csr = InstrFlow::Imm2RegCsr {
                imm: rs_1(instr) as u64,
                rd: rd(instr),
                csr: imm_i_type(instr) & 0xFFF,
            };

            let (op, flow) = match funct_3(instr) {
                0b000 => match imm_i_type(instr) {
                    0 => (InstrOp::EnvCall, InstrFlow::None),
                    1 => (InstrOp::EnvBreak, InstrFlow::None),
                    _ => match funct_7(instr) {
                        0b0001000 => match rs_2(instr) {
                            0b00010 => (InstrOp::SReturnFromTrap, InstrFlow::None),
                            0b00101 => (InstrOp::FenceI, InstrFlow::None), // wfi
                            _ => return None,
                        },
                        0b0001001 => (InstrOp::FenceI, InstrFlow::None),
                        0b0011000 => (InstrOp::MReturnFromTrap, InstrFlow::None),
                        _ => return None,
                    },
                },
                0b001 => (InstrOp::CsrReadAndWrite, reg_2_reg_csr),
                0b010 => (InstrOp::CsrReadAndSetBits, reg_2_reg_csr),
                0b011 => (InstrOp::CsrReadAndClearBits, reg_2_reg_csr),
                0b101 => (InstrOp::CsrReadAndWrite, imm_2_reg_csr),
                0b110 => (InstrOp::CsrReadAndSetBits, imm_2_reg_csr),
                0b111 => (InstrOp::CsrReadAndClearBits, imm_2_reg_csr),
                _ => return None,
            };

            (op, flow, OpWidth::None)
        }

        0b0101111 => {
            let reg_mem_2_reg = InstrFlow::RegMem2Reg {
                rs_1: rs_1(instr),
                rd: rd(instr),
            };

            let reg_reg_2_mem = InstrFlow::RegReg2RegMem {
                rs_1: rs_1(instr),
                rs_2: rs_2(instr),
                rd: rd(instr),
            };

            let reg_reg_mem_2_reg_mem = InstrFlow::RegRegMem2RegMem {
                rs_1: rs_1(instr),
                rs_2: rs_2(instr),
                rd: rd(instr),
            };

            let (op, flow, unsigned) = match funct_7_atomic_type(instr) {
                0b00010 => (InstrOp::LoadReserved, reg_mem_2_reg, false),
                0b00011 => (InstrOp::StoreConditional, reg_reg_2_mem, false),
                0b00001 => (InstrOp::AtomicSwap, reg_reg_mem_2_reg_mem, false),
                0b00000 => (InstrOp::AtomicAdd, reg_reg_mem_2_reg_mem, false),
                0b00100 => (InstrOp::AtomicXor, reg_reg_mem_2_reg_mem, false),
                0b01100 => (InstrOp::AtomicAnd, reg_reg_mem_2_reg_mem, false),
                0b01000 => (InstrOp::AtomicOr, reg_reg_mem_2_reg_mem, false),
                0b10000 => (InstrOp::AtomicMin, reg_reg_mem_2_reg_mem, false),
                0b10100 => (InstrOp::AtomicMax, reg_reg_mem_2_reg_mem, false),
                0b11000 => (InstrOp::AtomicMin, reg_reg_mem_2_reg_mem, true),
                0b11100 => (InstrOp::AtomicMax, reg_reg_mem_2_reg_mem, true),
                _ => return None,
            };

            let op_width = match (funct_3(instr), unsigned) {
                (0b010, false) => OpWidth::Word,
                (0b010, true) => OpWidth::WordUnsigned,
                (0b011, false) => OpWidth::DoubleWord,
                (0b011, true) => OpWidth::DoubleWordUnsigned,
                _ => return None,
            };

            (op, flow, op_width)
        }

        _ => return None,
    };

    Some(Instr {
        raw: instr,
        op,
        flow,
        op_width,
        instr_size: 4,
    })
}
