use std::collections::{HashMap, HashSet};

use anyhow::Result;
use walrus::{
    ir::{
        self, BinaryOp, Binop, Call, CallIndirect, Const, Drop, GlobalGet, GlobalSet, Instr,
        InstrSeq, InstrSeqId, LocalGet, LocalSet, LocalTee, MemArg, Store, StoreKind, TableGet,
        TableSet, Value, VisitorMut,
    },
    FunctionBuilder, FunctionKind, GlobalId, InstrLocId, InstrSeqBuilder, Local, LocalFunction,
    LocalId, MemoryId, Module, ModuleConfig, TableId, ValType,
};
use wasm_bindgen::prelude::*;

type Instruction = (Instr, InstrLocId);

#[wasm_bindgen]
pub fn instrument_wasm_js(buffer: &[u8]) -> Result<JsValue, JsValue> {
    let mut module = instrument_wasm(buffer).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let value = serde_wasm_bindgen::to_value(&module.emit_wasm())?;
    Ok(value)
}

pub fn instrument_wasm(buffer: &[u8]) -> Result<Module> {
    let mut module = Module::from_buffer(buffer)?;
    let trace_mem_id = module.memories.add_local(false, 100, None);
    let mem_pointer = module.globals.add_local(
        walrus::ValType::I32,
        true,
        walrus::InitExpr::Value(walrus::ir::Value::I32(0)),
    );
    let added_locals = add_locals(&mut module);
    let mut generator = Generator::new(trace_mem_id, mem_pointer, added_locals);
    module
        .funcs
        .iter_local_mut()
        .for_each(|(_, f)| ir::dfs_pre_order_mut(&mut generator, f, f.entry_block()));
    Ok(module)
}

type AddedLocals = HashMap<ValType, Vec<LocalId>>;

fn add_locals(module: &mut Module) -> HashMap<ValType, Vec<LocalId>> {
    let mut added_locals: AddedLocals = HashMap::new();
    [
        ValType::I32,
        ValType::I32,
        ValType::I64,
        ValType::F32,
        ValType::F64,
    ]
    .into_iter()
    .for_each(|t| {
        match added_locals.get_mut(&t) {
            Some(local) => local.push(module.locals.add(t)),
            None => {
                added_locals.insert(t, vec![module.locals.add(t)]);
            }
        };
    });
    added_locals
}

enum InstructionsEnum {
    Sequence(Vec<Instruction>),
    Single(Instruction),
}

impl InstructionsEnum {
    pub fn from_vec(vec: Vec<InstructionsEnum>) -> Self {
        Self::Sequence(
            vec.into_iter()
                .map(|e| match e {
                    InstructionsEnum::Sequence(s) => s,
                    InstructionsEnum::Single(s) => vec![s],
                })
                .flat_map(|s| s.into_iter())
                .collect(),
        )
    }

    pub fn flatten(&self) -> Vec<Instruction> {
        match self {
            InstructionsEnum::Sequence(s) => s.to_vec(),
            InstructionsEnum::Single(s) => vec![s.clone()],
        }
    }
}

#[derive(Debug)]
struct Generator {
    trace_mem_id: MemoryId,
    mem_pointer: GlobalId,
    added_locals: AddedLocals,
}

impl VisitorMut for Generator {
    fn visit_instr_seq_id_mut(&mut self, instr_seq_id: &mut InstrSeqId) {
        println!("{:?}", instr_seq_id);
    }

    fn start_instr_seq_mut(&mut self, seq: &mut ir::InstrSeq) {
        seq.clone().iter().enumerate().for_each(|(i, (instr, _))| {
            let gen_seq: Vec<Instruction>;
            match instr {
                Instr::Load(load) => {
                    let (opcode, local_type, byte_length) = match load.kind {
                        ir::LoadKind::I32 { .. } => (0x28, ValType::I32, 4),
                        ir::LoadKind::I64 { .. } => (0x29, ValType::I64, 8),
                        ir::LoadKind::F32 => (0x2A, ValType::F32, 4),
                        ir::LoadKind::F64 => (0x2B, ValType::F64, 8),
                        ir::LoadKind::V128 => todo!(),
                        ir::LoadKind::I32_8 { .. } => (0x2C, ValType::I32, 1),
                        ir::LoadKind::I32_16 { .. } => (0x2E, ValType::I32, 2),
                        ir::LoadKind::I64_8 { .. } => (0x30, ValType::I64, 1),
                        ir::LoadKind::I64_16 { .. } => (0x32, ValType::I64, 2),
                        ir::LoadKind::I64_32 { .. } => (0x34, ValType::I64, 4),
                    };
                    gen_seq = InstructionsEnum::from_vec(vec![
                        self.trace_code(opcode),
                        self.local_tee(
                            *self.added_locals.get(&local_type).unwrap().get(0).unwrap(),
                        ),
                        self.global_get(self.mem_pointer),
                        self.local_get(
                            *self.added_locals.get(&local_type).unwrap().get(0).unwrap(),
                        ),
                        self.store_to_trace(to_store_kind(byte_length), 1),
                        self.instr(instr.clone()),
                        self.local_tee(
                            *self
                                .added_locals
                                .get(&ValType::I32)
                                .unwrap()
                                .get(0)
                                .unwrap(),
                        ),
                        self.global_get(self.mem_pointer),
                        self.local_get(
                            *self.added_locals.get(&local_type).unwrap().get(0).unwrap(),
                        ),
                        self.store_to_trace(ir::StoreKind::I32 { atomic: false }, 1 + byte_length),
                        self.increment_mem_pointer(5 + byte_length as i32),
                    ])
                    .flatten();
                    seq.splice(i..(i + 1), gen_seq);
                }
                _ => {}
            };
        })
    }
}

impl Generator {
    fn new(trace_mem_id: MemoryId, mem_pointer: GlobalId, added_locals: AddedLocals) -> Self {
        Self {
            trace_mem_id,
            mem_pointer,
            added_locals,
        }
    }

    fn build(&mut self) {
        // self.module.funcs.iter_local_mut().for_each(|(_, f)| {
        //     let block = f.block_mut(f.entry_block());
        //     block
        //         .instrs
        //         .clone()
        //         .iter()
        //         .enumerate()
        //         .for_each(|(i, (instr, _))| match instr {
        //             Instr::Load(load) => {
        //                 let (opcode, locals, byte_length) = match load.kind {
        //                     ir::LoadKind::I32 { .. } => (0x28, &[ValType::I32, ValType::I32], 4),
        //                     ir::LoadKind::I64 { .. } => (0x29, &[ValType::I32, ValType::I64], 8),
        //                     ir::LoadKind::F32 => (0x2A, &[ValType::I32, ValType::F32], 4),
        //                     ir::LoadKind::F64 => (0x2B, &[ValType::I32, ValType::F64], 8),
        //                     ir::LoadKind::V128 => todo!(),
        //                     ir::LoadKind::I32_8 { .. } => (0x2C, &[ValType::I32, ValType::I32], 1),
        //                     ir::LoadKind::I32_16 { .. } => (0x2E, &[ValType::I32, ValType::I32], 2),
        //                     ir::LoadKind::I64_8 { .. } => (0x30, &[ValType::I32, ValType::I64], 1),
        //                     ir::LoadKind::I64_16 { .. } => (0x32, &[ValType::I32, ValType::I64], 2),
        //                     ir::LoadKind::I64_32 { .. } => (0x34, &[ValType::I32, ValType::I64], 4),
        //                 };
        //                 let locals = self.add_locals(locals);
        //                 self.trace_code(&mut block.instrs, opcode, i);
        //                 seq.local_tee(*locals.get(0).unwrap())
        //                     .global_get(self.mem_pointer)
        //                     .local_get(*locals.get(0).unwrap())
        //                     .store(
        //                         self.trace_mem_id,
        //                         to_store_kind(byte_length),
        //                         MemArg {
        //                             offset: 1,
        //                             align: 0,
        //                         },
        //                     )
        //                     .instr(load)
        //                     .local_tee(*locals.get(1).unwrap())
        //                     .global_get(self.mem_pointer)
        //                     .local_get(*locals.get(1).unwrap())
        //                     .store(
        //                         self.trace_mem_id,
        //                         ir::StoreKind::I32 { atomic: false },
        //                         MemArg {
        //                             offset: 1 + byte_length,
        //                             align: 0,
        //                         },
        //                     );
        //                 self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
        //                 // block.insert(i, (Instr::Drop(Drop {}), InstrLocId::default()));
        //                 // block.insert(
        //                 //     i,
        //                 //     (
        //                 //         Instr::Const(Const {
        //                 //             value: ir::Value::I32(0),
        //                 //         }),
        //                 //         InstrLocId::default(),
        //                 //     ),
        //                 // );
        //             }
        //             _ => {}
        //         });
        // });
        // let mut builder = FunctionBuilder::new(&mut self.module.types, &[], &[]);
        // let mut seq = builder.func_body();
        // let old_body = func.builder_mut().func_body();
        // let instrs = old_body.instrs();
        // for (instr_ref, _) in instrs {
        //     let instr = instr_ref.clone();
        //     match instr {
        //         Instr::Call(_) => {
        //             self.trace_call(&mut seq, instr);
        //         }
        //         Instr::CallIndirect(c) => {
        //             self.trace_table_get(&mut seq, c.table);
        //             self.trace_call(&mut seq, Instr::CallIndirect(c));
        //         }
        //         Instr::GlobalGet(get) => {
        //             // seq.i32_const(0);
        //             self.trace_code(&mut seq, 0x23);
        //             let global_type = self.module.globals.get(get.global).ty;
        //             let (store_kind, byte_length) = store_info(global_type);
        //             let locals = self.add_locals(&[global_type]);
        //             seq.global_get(self.mem_pointer)
        //                 .i32_const(get.global.index() as i32)
        //                 .store(
        //                     self.trace_mem_id,
        //                     StoreKind::I32 { atomic: false },
        //                     MemArg {
        //                         offset: 1,
        //                         align: 0,
        //                     },
        //                 )
        //                 .instr(GlobalGet { global: get.global })
        //                 .local_tee(*locals.get(0).unwrap())
        //                 .global_get(self.mem_pointer)
        //                 .local_get(*locals.get(0).unwrap())
        //                 .store(
        //                     self.trace_mem_id,
        //                     store_kind,
        //                     MemArg {
        //                         offset: 5,
        //                         align: 0,
        //                     },
        //                 );
        //             self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
        //         }
        //         Instr::GlobalSet(set) => {
        //             self.trace_code(&mut seq, 0x24);
        //             let global_type = self.module.globals.get(set.global).ty;
        //             let (store_kind, byte_length) = store_info(global_type);
        //             let locals = self.add_locals(&[global_type]);
        //             seq.local_set(*locals.get(0).unwrap())
        //                 .global_get(self.mem_pointer)
        //                 .local_get(*locals.get(0).unwrap())
        //                 .store(
        //                     self.trace_mem_id,
        //                     store_kind,
        //                     MemArg {
        //                         offset: 1,
        //                         align: 0,
        //                     },
        //                 )
        //                 .local_get(*locals.get(0).unwrap())
        //                 .instr(GlobalSet { global: set.global });
        //             self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
        //         }
        //         Instr::Return(_) => todo!(),
        //         Instr::MemoryGrow(_) => todo!(),
        //         Instr::MemoryInit(_) => todo!(),
        //         Instr::DataDrop(_) => todo!(),
        //         Instr::MemoryCopy(_) => todo!(),
        //         Instr::MemoryFill(_) => todo!(),
        //         Instr::Load(load) => {
        //             let (opcode, locals, byte_length) = match load.kind {
        //                 ir::LoadKind::I32 { .. } => (0x28, &[ValType::I32, ValType::I32], 4),
        //                 ir::LoadKind::I64 { .. } => (0x29, &[ValType::I32, ValType::I64], 8),
        //                 ir::LoadKind::F32 => (0x2A, &[ValType::I32, ValType::F32], 4),
        //                 ir::LoadKind::F64 => (0x2B, &[ValType::I32, ValType::F64], 8),
        //                 ir::LoadKind::V128 => todo!(),
        //                 ir::LoadKind::I32_8 { .. } => (0x2C, &[ValType::I32, ValType::I32], 1),
        //                 ir::LoadKind::I32_16 { .. } => (0x2E, &[ValType::I32, ValType::I32], 2),
        //                 ir::LoadKind::I64_8 { .. } => (0x30, &[ValType::I32, ValType::I64], 1),
        //                 ir::LoadKind::I64_16 { .. } => (0x32, &[ValType::I32, ValType::I64], 2),
        //                 ir::LoadKind::I64_32 { .. } => (0x34, &[ValType::I32, ValType::I64], 4),
        //             };
        //             let locals = self.add_locals(locals);
        //             self.trace_code(&mut seq, opcode);
        //             seq.local_tee(*locals.get(0).unwrap())
        //                 .global_get(self.mem_pointer)
        //                 .local_get(*locals.get(0).unwrap())
        //                 .store(
        //                     self.trace_mem_id,
        //                     to_store_kind(byte_length),
        //                     MemArg {
        //                         offset: 1,
        //                         align: 0,
        //                     },
        //                 )
        //                 .instr(load)
        //                 .local_tee(*locals.get(1).unwrap())
        //                 .global_get(self.mem_pointer)
        //                 .local_get(*locals.get(1).unwrap())
        //                 .store(
        //                     self.trace_mem_id,
        //                     ir::StoreKind::I32 { atomic: false },
        //                     MemArg {
        //                         offset: 1 + byte_length,
        //                         align: 0,
        //                     },
        //                 );
        //             self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
        //         }
        //         Instr::Store(store) => {
        //             let (opcode, locals, byte_length) = match store.kind {
        //                 ir::StoreKind::I32 { .. } => (0x36, &[ValType::I32, ValType::I32], 4),
        //                 ir::StoreKind::I64 { .. } => (0x37, &[ValType::I32, ValType::I64], 8),
        //                 ir::StoreKind::F32 => (0x38, &[ValType::I32, ValType::F32], 4),
        //                 ir::StoreKind::F64 => (0x39, &[ValType::I32, ValType::F64], 8),
        //                 ir::StoreKind::V128 => todo!(),
        //                 ir::StoreKind::I32_8 { .. } => (0x3A, &[ValType::I32, ValType::I32], 1),
        //                 ir::StoreKind::I32_16 { .. } => (0x3B, &[ValType::I32, ValType::I32], 2),
        //                 ir::StoreKind::I64_8 { .. } => (0x3C, &[ValType::I32, ValType::I64], 1),
        //                 ir::StoreKind::I64_16 { .. } => (0x3D, &[ValType::I32, ValType::I64], 2),
        //                 ir::StoreKind::I64_32 { .. } => (0x3E, &[ValType::I32, ValType::I64], 4),
        //             };
        //             let locals = self.add_locals(locals);
        //             self.trace_code(&mut seq, opcode);
        //             seq.global_get(self.mem_pointer)
        //                 .local_tee(*locals.get(0).unwrap())
        //                 .store(
        //                     self.trace_mem_id,
        //                     store.kind,
        //                     MemArg {
        //                         offset: 1,
        //                         align: 0,
        //                     },
        //                 )
        //                 .global_get(self.mem_pointer)
        //                 .local_tee(*locals.get(1).unwrap())
        //                 .store(
        //                     self.trace_mem_id,
        //                     ir::StoreKind::I32 { atomic: false },
        //                     MemArg {
        //                         offset: 1 + byte_length,
        //                         align: 0,
        //                     },
        //                 );
        //             self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
        //             seq.local_get(*locals.get(1).unwrap())
        //                 .local_get(*locals.get(0).unwrap())
        //                 .instr(store);
        //         }
        //         Instr::TableGet(get) => {
        //             self.trace_table_get(&mut seq, get.table);
        //         }
        //         Instr::TableSet(set) => {
        //             self.trace_code(&mut seq, 0x26);
        //             let table_type = self.module.tables.get(set.table).element_ty;
        //             let locals = self.add_locals(&[ValType::I32, table_type]);
        //             seq.global_get(self.mem_pointer)
        //                 .local_tee(*locals.get(0).unwrap())
        //                 .store(
        //                     self.trace_mem_id,
        //                     StoreKind::I32 { atomic: false },
        //                     MemArg {
        //                         offset: 1,
        //                         align: 0,
        //                     },
        //                 )
        //                 .instr(TableSet { table: set.table })
        //                 .local_set(*locals.get(1).unwrap())
        //                 .global_get(self.mem_pointer)
        //                 .local_get(*locals.get(1).unwrap())
        //                 .store(
        //                     self.trace_mem_id,
        //                     store_info(table_type).0,
        //                     MemArg {
        //                         offset: 5,
        //                         align: 0,
        //                     },
        //                 )
        //                 .local_get(*locals.get(1).unwrap());
        //             self.increment_mem_pointer(&mut seq, 5 + store_info(table_type).1 as i32);
        //         }
        //         Instr::TableGrow(_) => todo!(),
        //         Instr::TableFill(_) => todo!(),
        //         Instr::LoadSimd(_) => todo!(),
        //         Instr::TableInit(_) => todo!(),
        //         Instr::ElemDrop(_) => todo!(),
        //         Instr::TableCopy(_) => todo!(),
        //         // Instr::Block(b) => seq.instr(Instr::Block {
        //         //     seq: self.build_sequence(b.seq),
        //         // }),
        //         // Instr::IfElse(if_else) => {
        //         //     seq.instr(if_else);
        //         // }
        //         // Instr::Loop(l) => {
        //         //     seq.instr(l);
        //         // }
        //         _ => {
        //             seq.instr(instr);
        //         }
        //     }
        // }
        // builder.finish(func.args.clone(), &mut self.module.funcs);
    }

    fn trace_code(&self, code: i32) -> InstructionsEnum {
        InstructionsEnum::from_vec(vec![
            self.global_get(self.mem_pointer),
            self.get_const(Value::I32(code)),
            self.store_to_trace(StoreKind::I32_8 { atomic: false }, 0),
        ])
    }

    fn store_to_trace(&self, kind: StoreKind, offset: u32) -> InstructionsEnum {
        InstructionsEnum::Single((
            Instr::Store(Store {
                memory: self.trace_mem_id,
                kind,
                arg: MemArg { align: 0, offset },
            }),
            InstrLocId::default(),
        ))
    }

    fn instr(&self, instr: Instr) -> InstructionsEnum {
        InstructionsEnum::Single((instr, InstrLocId::default()))
    }

    fn get_const(&self, value: Value) -> InstructionsEnum {
        InstructionsEnum::Single((Instr::Const(Const { value }), InstrLocId::default()))
    }

    fn global_get(&self, global: GlobalId) -> InstructionsEnum {
        InstructionsEnum::Single((
            Instr::GlobalGet(GlobalGet { global }),
            InstrLocId::default(),
        ))
    }

    fn global_set(&self, global: GlobalId) -> InstructionsEnum {
        InstructionsEnum::Single((
            Instr::GlobalSet(GlobalSet { global }),
            InstrLocId::default(),
        ))
    }

    fn local_tee(&self, local: LocalId) -> InstructionsEnum {
        InstructionsEnum::Single((Instr::LocalTee(LocalTee { local }), InstrLocId::default()))
    }

    fn local_get(&self, local: LocalId) -> InstructionsEnum {
        InstructionsEnum::Single((Instr::LocalGet(LocalGet { local }), InstrLocId::default()))
    }

    fn local_set(&self, local: LocalId) -> InstructionsEnum {
        InstructionsEnum::Single((Instr::LocalSet(LocalSet { local }), InstrLocId::default()))
    }

    fn increment_mem_pointer(&self, amount: i32) -> InstructionsEnum {
        InstructionsEnum::from_vec(vec![
            self.global_get(self.mem_pointer),
            self.get_const(Value::I32(amount)),
            self.binop(ir::BinaryOp::I32Add),
            self.global_set(self.mem_pointer),
        ])
    }

    fn binop(&self, op: BinaryOp) -> InstructionsEnum {
        InstructionsEnum::Single((Instr::Binop(Binop { op }), InstrLocId::default()))
    }

    //     fn trace_call(&mut self, seq: &mut InstrSeqBuilder, call: Instr) {
    //         let (call, opcode, type_id) = match call {
    //             Instr::Call(c) => (
    //                 Instr::Call(Call { func: c.func }),
    //                 0x10,
    //                 self.module.funcs.get(c.func).ty(),
    //             ),
    //             Instr::CallIndirect(c) => (
    //                 Instr::CallIndirect(CallIndirect {
    //                     ty: c.ty,
    //                     table: c.table,
    //                 }),
    //                 0x11,
    //                 c.ty,
    //             ),
    //             _ => panic!(
    //                 "You are not allowed to call the function `trace_call` with a non call instruction"
    //             ),
    //         };
    //         let typ = self.module.types.get(type_id).clone();
    //         let type_id: i32 = type_id.index() as i32;
    //         let params = self.add_locals(typ.params());
    //         let results = self.add_locals(typ.results());
    //         self.trace_code(seq, opcode);
    //         seq.global_get(self.mem_pointer).i32_const(type_id).store(
    //             self.trace_mem_id,
    //             ir::StoreKind::I32 { atomic: false },
    //             MemArg {
    //                 offset: 1,
    //                 align: 0,
    //             },
    //         );
    //         let mut offset = 5;
    //         params
    //             .into_iter()
    //             .map(|p| {
    //                 let (store_kind, byte_length) = store_info(self.module.locals.get(p).ty());
    //                 seq.global_get(self.mem_pointer).local_tee(p).store(
    //                     self.trace_mem_id,
    //                     store_kind,
    //                     MemArg { offset, align: 0 },
    //                 );
    //                 offset += byte_length;
    //                 p
    //             })
    //             .collect::<Vec<_>>()
    //             .into_iter()
    //             .for_each(|p| {
    //                 seq.local_get(p);
    //             });
    //         seq.instr(call);
    //         results
    //             .into_iter()
    //             .map(|r| {
    //                 let (store_kind, byte_length) = store_info(self.module.locals.get(r).ty());
    //                 seq.global_get(self.mem_pointer).local_tee(r).store(
    //                     self.trace_mem_id,
    //                     store_kind,
    //                     MemArg { offset, align: 0 },
    //                 );
    //                 offset += byte_length;
    //                 r
    //             })
    //             .collect::<Vec<_>>()
    //             .into_iter()
    //             .for_each(|r| {
    //                 seq.local_get(r);
    //             });
    //         self.increment_mem_pointer(seq, offset as i32);
    //     }

    //     fn trace_table_get(&mut self, seq: &mut InstrSeqBuilder, table_id: TableId) {
    //         let opcode = 0x25;
    //         self.trace_code(seq, opcode);
    //         let table_type = self.module.tables.get(table_id).element_ty;

    //         let locals = self.add_locals(&[ValType::I32, table_type]);
    //         seq.global_get(self.mem_pointer)
    //             .local_tee(*locals.get(0).unwrap())
    //             .store(
    //                 self.trace_mem_id,
    //                 StoreKind::I32 { atomic: false },
    //                 MemArg {
    //                     align: 0,
    //                     offset: 1,
    //                 },
    //             )
    //             .local_get(*locals.get(0).unwrap())
    //             .instr(TableGet { table: table_id })
    //             .global_get(self.mem_pointer)
    //             .local_set(*locals.get(0).unwrap())
    //             .store(
    //                 self.trace_mem_id,
    //                 store_info(table_type).0,
    //                 MemArg {
    //                     align: 0,
    //                     offset: 5,
    //                 },
    //             )
    //             .local_get(*locals.get(0).unwrap());
    //         self.increment_mem_pointer(seq, 5 + store_info(table_type).1 as i32);
    //     }
}

fn to_store_kind(byte_length: u32) -> StoreKind {
    match byte_length {
        1 => StoreKind::I32_8 { atomic: false },
        2 => StoreKind::I32_16 { atomic: false },
        4 => StoreKind::I32 { atomic: false },
        8 => StoreKind::I64 { atomic: false },
        _ => panic!(),
    }
}

fn store_info(local_type: ValType) -> (StoreKind, u32) {
    match local_type {
        ValType::I32 => (StoreKind::I32 { atomic: false }, 4),
        ValType::I64 => (StoreKind::I64 { atomic: false }, 8),
        ValType::F32 => (StoreKind::F32, 4),
        ValType::F64 => (StoreKind::F64, 8),
        ValType::V128 => todo!(),
        ValType::Externref => (StoreKind::I32 { atomic: false }, 4),
        ValType::Funcref => (StoreKind::I32 { atomic: false }, 4),
    }
}
