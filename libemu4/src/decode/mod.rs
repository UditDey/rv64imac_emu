mod decode_16;
mod decode_32;

fn sign_extend(val: u64, sign_bit_pos: u64) -> u64 {
    let shift = 64 - (sign_bit_pos + 1);
    (((val << shift) as i64) >> shift) as u64
}

#[derive(Debug, Clone, Copy)]
pub struct Instr {
    pub raw: u32,
    pub op: InstrOp,
    pub flow: InstrFlow,
    pub op_width: OpWidth,
    pub instr_size: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum InstrOp {
    LoadUpperImm,
    AddUpperImmAndPC,

    JumpAndLink,
    JumpAndLinkReg,

    BranchIfEquals,
    BranchIfNotEquals,
    BranchIfLesserThan,
    BranchIfGreaterThanOrEquals,

    Load,
    Store,

    Add,
    Multiply,
    Subtract,
    ShiftLeft,
    MultiplyHigh,
    SetLesserThan,
    Xor,
    Divide,
    ShiftRightLogical,
    ShiftRightArithmetic,
    Or,
    Remainder,
    And,

    FenceI,
    EnvCall,
    EnvBreak,
    SReturnFromTrap,
    MReturnFromTrap,

    CsrReadAndWrite,
    CsrReadAndSetBits,
    CsrReadAndClearBits,

    LoadReserved,
    StoreConditional,
    AtomicSwap,
    AtomicAdd,
    AtomicXor,
    AtomicAnd,
    AtomicOr,
    AtomicMin,
    AtomicMax,
}

#[derive(Debug, Clone, Copy)]
pub enum OpWidth {
    None,
    DoubleWord,
    Word,
    HalfWord,
    Byte,
    DoubleWordUnsigned,
    DoubleWordSignedUnsigned,
    WordUnsigned,
    HalfWordUnsigned,
    ByteUnsigned,
}

#[derive(Debug, Clone, Copy)]
pub enum InstrFlow {
    None,

    // lui/auipc
    Imm2Reg { imm: u64, rd: usize },
    ImmPC2Reg { imm: u64, rd: usize },

    // jal/jalr
    ImmPC2RegPC { imm: u64, rd: usize },
    RegImmPC2RegPC { rs_1: usize, imm: u64, rd: usize },

    // branch
    RegRegImm2PC { rs_1: usize, rs_2: usize, imm: u64 },

    RegImmMem2Reg { rs_1: usize, imm: u64, rd: usize },
    RegRegImm2Mem { rs_1: usize, rs_2: usize, imm: u64 },

    RegMem2Reg { rs_1: usize, rd: usize },
    RegReg2RegMem { rs_1: usize, rs_2: usize, rd: usize },
    RegRegMem2RegMem { rs_1: usize, rs_2: usize, rd: usize },

    RegReg2Reg { rs_1: usize, rs_2: usize, rd: usize },
    RegImm2Reg { rs_1: usize, imm: u64, rd: usize },

    Reg2RegCsr { rs_1: usize, rd: usize, csr: u64 },
    Imm2RegCsr { imm: u64, rd: usize, csr: u64 },
}

pub fn decode(instr: u32) -> Option<Instr> {
    match instr & 0b11 {
        0b11 => decode_32::decode(instr),
        _ => decode_16::decode(instr as u16),
    }
}
