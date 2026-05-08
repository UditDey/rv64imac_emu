use std::fs::File;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use arrow::{
    array::{ArrayBuilder, BooleanBuilder, Int64Builder, RecordBatch, UInt8Builder, UInt64Builder},
    datatypes::{DataType, Field, Schema},
};

use parquet::{
    arrow::ArrowWriter,
    basic::{Compression, Encoding, ZstdLevel},
    file::properties::WriterProperties,
    schema::types::ColumnPath,
};

use libemu4::decode::{Instr, InstrFlow, InstrOp, OpWidth};

const BATCH_SIZE: usize = 10_000_000;

enum Command {
    Batch(RecordBatch),
    FinalBatch(RecordBatch),
}

pub struct Tracer {
    schema: Arc<Schema>,
    cycle: UInt64Builder,
    pc: UInt64Builder,
    compressed: BooleanBuilder,
    op: UInt8Builder,
    op_width: UInt8Builder,
    op_flow: UInt8Builder,
    rd: UInt8Builder,
    rs_1: UInt8Builder,
    rs_2: UInt8Builder,
    imm: Int64Builder,
    rs_1_val: Int64Builder,
    rs_2_val: Int64Builder,
    writer: JoinHandle<()>,
    tx: Sender<Command>,
}

impl Tracer {
    pub fn new() -> Self {
        let schema = Arc::new(Schema::new(vec![
            Field::new("cycle", DataType::UInt64, false),
            Field::new("pc", DataType::UInt64, false),
            Field::new("compressed", DataType::Boolean, false),
            Field::new("op_raw", DataType::UInt8, true),
            Field::new("op_width_raw", DataType::UInt8, true),
            Field::new("flow_raw", DataType::UInt8, true),
            Field::new("rd", DataType::UInt8, true),
            Field::new("rs_1", DataType::UInt8, true),
            Field::new("rs_2", DataType::UInt8, true),
            Field::new("imm", DataType::Int64, true),
            Field::new("rs_1_val", DataType::Int64, true),
            Field::new("rs_2_val", DataType::Int64, true),
        ]));

        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open("../trace.pqt")
            .unwrap();

        let props = WriterProperties::builder()
            .set_column_encoding(ColumnPath::from("cycle"), Encoding::DELTA_BINARY_PACKED)
            .set_column_encoding(ColumnPath::from("pc"), Encoding::DELTA_BINARY_PACKED)
            .set_compression(Compression::ZSTD(ZstdLevel::try_new(1).unwrap()))
            .build();

        let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props)).unwrap();

        let (tx, rx) = std::sync::mpsc::channel();

        let writer = std::thread::spawn(move || {
            for cmd in rx.iter() {
                match cmd {
                    Command::Batch(batch) => {
                        writer.write(&batch).unwrap();
                    }
                    Command::FinalBatch(batch) => {
                        writer.write(&batch).unwrap();
                        writer.close().unwrap();
                        break;
                    }
                }
            }
        });

        Self {
            schema,
            cycle: UInt64Builder::new(),
            pc: UInt64Builder::new(),
            compressed: BooleanBuilder::new(),
            op: UInt8Builder::new(),
            op_width: UInt8Builder::new(),
            op_flow: UInt8Builder::new(),
            rd: UInt8Builder::new(),
            rs_1: UInt8Builder::new(),
            rs_2: UInt8Builder::new(),
            imm: Int64Builder::new(),
            rs_1_val: Int64Builder::new(),
            rs_2_val: Int64Builder::new(),
            writer,
            tx,
        }
    }

    pub fn trace(&mut self, instr: Instr, cycle: u64, pc: u64, regs: &[u64; 32]) {
        if self.cycle.len() == BATCH_SIZE {
            let columns = vec![
                ArrayBuilder::finish(&mut self.cycle),
                ArrayBuilder::finish(&mut self.pc),
                ArrayBuilder::finish(&mut self.compressed),
                ArrayBuilder::finish(&mut self.op),
                ArrayBuilder::finish(&mut self.op_width),
                ArrayBuilder::finish(&mut self.op_flow),
                ArrayBuilder::finish(&mut self.rd),
                ArrayBuilder::finish(&mut self.rs_1),
                ArrayBuilder::finish(&mut self.rs_2),
                ArrayBuilder::finish(&mut self.imm),
                ArrayBuilder::finish(&mut self.rs_1_val),
                ArrayBuilder::finish(&mut self.rs_2_val),
            ];

            let batch = RecordBatch::try_new(self.schema.clone(), columns).unwrap();
            self.tx.send(Command::Batch(batch)).unwrap();
        }

        let op = match instr.op {
            InstrOp::LoadUpperImm => 0,
            InstrOp::AddUpperImmAndPC => 1,
            InstrOp::JumpAndLink => 2,
            InstrOp::JumpAndLinkReg => 3,
            InstrOp::BranchIfEquals => 4,
            InstrOp::BranchIfNotEquals => 5,
            InstrOp::BranchIfLesserThan => 6,
            InstrOp::BranchIfGreaterThanOrEquals => 7,
            InstrOp::Load => 8,
            InstrOp::Store => 9,
            InstrOp::Add => 10,
            InstrOp::Multiply => 11,
            InstrOp::Subtract => 12,
            InstrOp::ShiftLeft => 13,
            InstrOp::MultiplyHigh => 14,
            InstrOp::SetLesserThan => 15,
            InstrOp::Xor => 16,
            InstrOp::Divide => 17,
            InstrOp::ShiftRightLogical => 18,
            InstrOp::ShiftRightArithmetic => 19,
            InstrOp::Or => 20,
            InstrOp::Remainder => 21,
            InstrOp::And => 22,
            InstrOp::FenceI => 23,
            InstrOp::EnvCall => 24,
            InstrOp::EnvBreak => 25,
            InstrOp::SReturnFromTrap => 26,
            InstrOp::MReturnFromTrap => 27,
            InstrOp::CsrReadAndWrite => 28,
            InstrOp::CsrReadAndSetBits => 29,
            InstrOp::CsrReadAndClearBits => 30,
            InstrOp::LoadReserved => 31,
            InstrOp::StoreConditional => 32,
            InstrOp::AtomicSwap => 33,
            InstrOp::AtomicAdd => 34,
            InstrOp::AtomicXor => 35,
            InstrOp::AtomicAnd => 36,
            InstrOp::AtomicOr => 37,
            InstrOp::AtomicMin => 38,
            InstrOp::AtomicMax => 39,
        };

        let op_width = match instr.op_width {
            OpWidth::None => None,
            OpWidth::DoubleWord => Some(0),
            OpWidth::Word => Some(1),
            OpWidth::HalfWord => Some(2),
            OpWidth::Byte => Some(3),
            OpWidth::DoubleWordUnsigned => Some(4),
            OpWidth::DoubleWordSignedUnsigned => Some(5),
            OpWidth::WordUnsigned => Some(6),
            OpWidth::HalfWordUnsigned => Some(7),
            OpWidth::ByteUnsigned => Some(8),
        };

        let (flow, rd, rs_1, rs_2, imm) = match instr.flow {
            InstrFlow::None => (None, None, None, None, None),
            InstrFlow::Imm2Reg { imm, rd } => {
                (Some(0), Some(rd as u8), None, None, Some(imm as i64))
            }
            InstrFlow::ImmPC2Reg { imm, rd } => {
                (Some(1), Some(rd as u8), None, None, Some(imm as i64))
            }
            InstrFlow::ImmPC2RegPC { imm, rd } => {
                (Some(2), Some(rd as u8), None, None, Some(imm as i64))
            }
            InstrFlow::RegImmPC2RegPC { rs_1, imm, rd } => (
                Some(3),
                Some(rd as u8),
                Some(rs_1 as u8),
                None,
                Some(imm as i64),
            ),
            InstrFlow::RegRegImm2PC { rs_1, rs_2, imm } => (
                Some(4),
                None,
                Some(rs_1 as u8),
                Some(rs_2 as u8),
                Some(imm as i64),
            ),
            InstrFlow::RegImmMem2Reg { rs_1, imm, rd } => (
                Some(5),
                Some(rd as u8),
                Some(rs_1 as u8),
                None,
                Some(imm as i64),
            ),
            InstrFlow::RegRegImm2Mem { rs_1, rs_2, imm } => (
                Some(6),
                None,
                Some(rs_1 as u8),
                Some(rs_2 as u8),
                Some(imm as i64),
            ),
            InstrFlow::RegMem2Reg { rs_1, rd } => {
                (Some(7), Some(rd as u8), Some(rs_1 as u8), None, None)
            }
            InstrFlow::RegReg2RegMem { rs_1, rs_2, rd } => (
                Some(8),
                Some(rd as u8),
                Some(rs_1 as u8),
                Some(rs_2 as u8),
                None,
            ),
            InstrFlow::RegRegMem2RegMem { rs_1, rs_2, rd } => (
                Some(9),
                Some(rd as u8),
                Some(rs_1 as u8),
                Some(rs_2 as u8),
                None,
            ),
            InstrFlow::RegReg2Reg { rs_1, rs_2, rd } => (
                Some(10),
                Some(rd as u8),
                Some(rs_1 as u8),
                Some(rs_2 as u8),
                None,
            ),
            InstrFlow::RegImm2Reg { rs_1, imm, rd } => (
                Some(11),
                Some(rd as u8),
                Some(rs_1 as u8),
                None,
                Some(imm as i64),
            ),
            InstrFlow::Reg2RegCsr {
                rs_1: _,
                rd: _,
                csr,
            } => (Some(12), None, None, None, Some(csr as i64)),
            InstrFlow::Imm2RegCsr { imm: _, rd: _, csr } => {
                (Some(13), None, None, None, Some(csr as i64))
            }
        };

        self.cycle.append_value(cycle);
        self.pc.append_value(pc);
        self.compressed.append_value(instr.instr_size == 2);
        self.op.append_value(op);
        self.op_width.append_option(op_width);
        self.op_flow.append_option(flow);
        self.rd.append_option(rd);
        self.rs_1.append_option(rs_1);
        self.rs_2.append_option(rs_2);
        self.imm.append_option(imm);
        self.rs_1_val
            .append_option(rs_1.map(|rs_1| regs[rs_1 as usize] as i64));
        self.rs_2_val
            .append_option(rs_2.map(|rs_2| regs[rs_2 as usize] as i64));
    }

    pub fn finish(mut self) {
        let columns = vec![
            ArrayBuilder::finish(&mut self.cycle),
            ArrayBuilder::finish(&mut self.pc),
            ArrayBuilder::finish(&mut self.compressed),
            ArrayBuilder::finish(&mut self.op),
            ArrayBuilder::finish(&mut self.op_width),
            ArrayBuilder::finish(&mut self.op_flow),
            ArrayBuilder::finish(&mut self.rd),
            ArrayBuilder::finish(&mut self.rs_1),
            ArrayBuilder::finish(&mut self.rs_2),
            ArrayBuilder::finish(&mut self.imm),
            ArrayBuilder::finish(&mut self.rs_1_val),
            ArrayBuilder::finish(&mut self.rs_2_val),
        ];

        let batch = RecordBatch::try_new(self.schema.clone(), columns).unwrap();

        self.tx.send(Command::FinalBatch(batch)).unwrap();
        self.writer.join().unwrap();
    }
}
