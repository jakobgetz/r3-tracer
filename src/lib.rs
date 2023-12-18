use anyhow::Result;
use walrus::{
    ir::{
        self, Call, CallIndirect, GlobalGet, GlobalSet, Instr, MemArg, StoreKind, TableGet,
        TableSet,
    },
    FunctionBuilder, GlobalId, InstrSeqBuilder, Local, LocalFunction, LocalId, MemoryId, Module,
    ModuleConfig, TableId, ValType,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn instrument_wasm_js(buffer: &[u8]) -> Result<JsValue, JsValue> {
    let mut module = instrument_wasm(buffer).map_err(|e| JsValue::from_str(&e.to_string()))?;
    println!("we come until here");
    let value = serde_wasm_bindgen::to_value(&module.emit_wasm())?;
    Ok(value)
}

pub fn instrument_wasm(buffer: &[u8]) -> Result<Module> {
    let mut module = Module::from_buffer(buffer)?;
    let mut new_module = Module::with_config(ModuleConfig::new());
    new_module.memories = module.memories;
    new_module.locals = module.locals;
    new_module.tables = module.tables;
    new_module.globals = module.globals;
    let mut generator = Generator::new(new_module);
    println!("so far");
    module
        .funcs
        .iter_local_mut()
        .for_each(|(_, f)| generator.build(f));
    println!("here it works still");
    println!("{:?}", generator.module);
    let mody = generator.module;
    println!("Hey");
    Ok(mody)
}

#[derive(Debug)]
struct Generator {
    trace_mem_id: MemoryId,
    mem_pointer: GlobalId,
    added_locals: Vec<Local>,
    pub module: Module,
}

impl Generator {
    fn new(mut module: Module) -> Self {
        Self {
            trace_mem_id: module.memories.add_local(false, 100, None),
            mem_pointer: module.globals.add_local(
                walrus::ValType::I32,
                true,
                walrus::InitExpr::Value(walrus::ir::Value::I32(0)),
            ),
            module,
            added_locals: Vec::new(),
        }
    }

    fn build(&mut self, func: &mut LocalFunction) {
        let mut builder = FunctionBuilder::new(&mut self.module.types, &[], &[]);
        let mut seq = builder.func_body();
        let body = func.builder_mut().func_body();
        let instrs = body.instrs();
        for (instr_ref, _) in instrs {
            let instr = instr_ref.clone();
            match instr {
                // Instr::Call(_) => {
                //     self.trace_call(&mut seq, instr);
                // }
                // Instr::CallIndirect(c) => {
                //     self.trace_table_get(&mut seq, c.table);
                //     self.trace_call(&mut seq, Instr::CallIndirect(c));
                // }
                // Instr::GlobalGet(get) => {
                //     // seq.i32_const(0);
                //     self.trace_code(&mut seq, 0x23);
                //     let global_type = self.module.globals.get(get.global).ty;
                //     let (store_kind, byte_length) = store_info(global_type);
                //     let locals = self.add_locals(&[global_type]);
                //     seq.global_get(self.mem_pointer)
                //         .i32_const(get.global.index() as i32)
                //         .store(
                //             self.trace_mem_id,
                //             StoreKind::I32 { atomic: false },
                //             MemArg {
                //                 offset: 1,
                //                 align: 0,
                //             },
                //         )
                //         .instr(GlobalGet { global: get.global })
                //         .local_tee(*locals.get(0).unwrap())
                //         .global_get(self.mem_pointer)
                //         .local_get(*locals.get(0).unwrap())
                //         .store(
                //             self.trace_mem_id,
                //             store_kind,
                //             MemArg {
                //                 offset: 5,
                //                 align: 0,
                //             },
                //         );
                //     self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
                // }
                // Instr::GlobalSet(set) => {
                //     self.trace_code(&mut seq, 0x24);
                //     let global_type = self.module.globals.get(set.global).ty;
                //     let (store_kind, byte_length) = store_info(global_type);
                //     let locals = self.add_locals(&[global_type]);
                //     seq.local_set(*locals.get(0).unwrap())
                //         .global_get(self.mem_pointer)
                //         .local_get(*locals.get(0).unwrap())
                //         .store(
                //             self.trace_mem_id,
                //             store_kind,
                //             MemArg {
                //                 offset: 1,
                //                 align: 0,
                //             },
                //         )
                //         .local_get(*locals.get(0).unwrap())
                //         .instr(GlobalSet { global: set.global });
                //     self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
                // }
                // Instr::Return(_) => todo!(),
                // Instr::MemoryGrow(_) => todo!(),
                // Instr::MemoryInit(_) => todo!(),
                // Instr::DataDrop(_) => todo!(),
                // Instr::MemoryCopy(_) => todo!(),
                // Instr::MemoryFill(_) => todo!(),
                // Instr::Load(load) => {
                //     let (opcode, locals, byte_length) = match load.kind {
                //         ir::LoadKind::I32 { .. } => (0x28, &[ValType::I32, ValType::I32], 4),
                //         ir::LoadKind::I64 { .. } => (0x29, &[ValType::I32, ValType::I64], 8),
                //         ir::LoadKind::F32 => (0x2A, &[ValType::I32, ValType::F32], 4),
                //         ir::LoadKind::F64 => (0x2B, &[ValType::I32, ValType::F64], 8),
                //         ir::LoadKind::V128 => todo!(),
                //         ir::LoadKind::I32_8 { .. } => (0x2C, &[ValType::I32, ValType::I32], 1),
                //         ir::LoadKind::I32_16 { .. } => (0x2E, &[ValType::I32, ValType::I32], 2),
                //         ir::LoadKind::I64_8 { .. } => (0x30, &[ValType::I32, ValType::I64], 1),
                //         ir::LoadKind::I64_16 { .. } => (0x32, &[ValType::I32, ValType::I64], 2),
                //         ir::LoadKind::I64_32 { .. } => (0x34, &[ValType::I32, ValType::I64], 4),
                //     };
                //     let locals = self.add_locals(locals);
                //     self.trace_code(&mut seq, opcode);
                //     seq.local_tee(*locals.get(0).unwrap())
                //         .global_get(self.mem_pointer)
                //         .local_get(*locals.get(0).unwrap())
                //         .store(
                //             self.trace_mem_id,
                //             to_store_kind(byte_length),
                //             MemArg {
                //                 offset: 1,
                //                 align: 0,
                //             },
                //         )
                //         .instr(load)
                //         .local_tee(*locals.get(1).unwrap())
                //         .global_get(self.mem_pointer)
                //         .local_get(*locals.get(1).unwrap())
                //         .store(
                //             self.trace_mem_id,
                //             ir::StoreKind::I32 { atomic: false },
                //             MemArg {
                //                 offset: 1 + byte_length,
                //                 align: 0,
                //             },
                //         );
                //     self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
                // }
                // Instr::Store(store) => {
                //     let (opcode, locals, byte_length) = match store.kind {
                //         ir::StoreKind::I32 { .. } => (0x36, &[ValType::I32, ValType::I32], 4),
                //         ir::StoreKind::I64 { .. } => (0x37, &[ValType::I32, ValType::I64], 8),
                //         ir::StoreKind::F32 => (0x38, &[ValType::I32, ValType::F32], 4),
                //         ir::StoreKind::F64 => (0x39, &[ValType::I32, ValType::F64], 8),
                //         ir::StoreKind::V128 => todo!(),
                //         ir::StoreKind::I32_8 { .. } => (0x3A, &[ValType::I32, ValType::I32], 1),
                //         ir::StoreKind::I32_16 { .. } => (0x3B, &[ValType::I32, ValType::I32], 2),
                //         ir::StoreKind::I64_8 { .. } => (0x3C, &[ValType::I32, ValType::I64], 1),
                //         ir::StoreKind::I64_16 { .. } => (0x3D, &[ValType::I32, ValType::I64], 2),
                //         ir::StoreKind::I64_32 { .. } => (0x3E, &[ValType::I32, ValType::I64], 4),
                //     };
                //     let locals = self.add_locals(locals);
                //     self.trace_code(&mut seq, opcode);
                //     seq.global_get(self.mem_pointer)
                //         .local_tee(*locals.get(0).unwrap())
                //         .store(
                //             self.trace_mem_id,
                //             store.kind,
                //             MemArg {
                //                 offset: 1,
                //                 align: 0,
                //             },
                //         )
                //         .global_get(self.mem_pointer)
                //         .local_tee(*locals.get(1).unwrap())
                //         .store(
                //             self.trace_mem_id,
                //             ir::StoreKind::I32 { atomic: false },
                //             MemArg {
                //                 offset: 1 + byte_length,
                //                 align: 0,
                //             },
                //         );
                //     self.increment_mem_pointer(&mut seq, 5 + byte_length as i32);
                //     seq.local_get(*locals.get(1).unwrap())
                //         .local_get(*locals.get(0).unwrap())
                //         .instr(store);
                // }
                // Instr::TableGet(get) => {
                //     self.trace_table_get(&mut seq, get.table);
                // }
                // Instr::TableSet(set) => {
                //     self.trace_code(&mut seq, 0x26);
                //     let table_type = self.module.tables.get(set.table).element_ty;
                //     let locals = self.add_locals(&[ValType::I32, table_type]);
                //     seq.global_get(self.mem_pointer)
                //         .local_tee(*locals.get(0).unwrap())
                //         .store(
                //             self.trace_mem_id,
                //             StoreKind::I32 { atomic: false },
                //             MemArg {
                //                 offset: 1,
                //                 align: 0,
                //             },
                //         )
                //         .instr(TableSet { table: set.table })
                //         .local_set(*locals.get(1).unwrap())
                //         .global_get(self.mem_pointer)
                //         .local_get(*locals.get(1).unwrap())
                //         .store(
                //             self.trace_mem_id,
                //             store_info(table_type).0,
                //             MemArg {
                //                 offset: 5,
                //                 align: 0,
                //             },
                //         )
                //         .local_get(*locals.get(1).unwrap());
                //     self.increment_mem_pointer(&mut seq, 5 + store_info(table_type).1 as i32);
                // }
                // Instr::TableGrow(_) => todo!(),
                // Instr::TableFill(_) => todo!(),
                // Instr::LoadSimd(_) => todo!(),
                // Instr::TableInit(_) => todo!(),
                // Instr::ElemDrop(_) => todo!(),
                // Instr::TableCopy(_) => todo!(),
                _ => {
                    seq.instr(instr);
                }
            }
        }
        builder.finish(func.args.clone(), &mut self.module.funcs);
    }

    fn add_locals(&mut self, types: &[ValType]) -> Vec<LocalId> {
        let mut unused_locals = self.added_locals.clone();
        types
            .into_iter()
            .map(|typ| {
                let local = match unused_locals.clone().into_iter().find(|l| l.ty() == *typ) {
                    Some(local) => local,
                    None => {
                        let id = self.module.locals.add(*typ);
                        self.module.locals.get(id).clone()
                    }
                };
                if unused_locals.len() > 0 {
                    let mut i = 0;
                    for l in &unused_locals {
                        if l == &local {
                            break;
                        }
                        i += 1;
                    }
                    unused_locals.remove(i);
                }
                self.added_locals.push(local.clone());
                local.id()
            })
            .collect()
    }

    fn trace_code(&self, seq: &mut InstrSeqBuilder, code: i32) {
        seq.global_get(self.mem_pointer).i32_const(code).store(
            self.trace_mem_id,
            ir::StoreKind::I32_8 { atomic: false },
            MemArg {
                offset: 0,
                align: 0,
            },
        );
    }

    fn increment_mem_pointer(&self, seq: &mut InstrSeqBuilder, amount: i32) {
        seq.global_get(self.mem_pointer)
            .i32_const(amount)
            .binop(ir::BinaryOp::I32Add)
            .global_set(self.mem_pointer);
    }

    fn trace_call(&mut self, seq: &mut InstrSeqBuilder, call: Instr) {
        let (call, opcode, type_id) = match call {
            Instr::Call(c) => (
                Instr::Call(Call { func: c.func }),
                0x10,
                self.module.funcs.get(c.func).ty(),
            ),
            Instr::CallIndirect(c) => (
                Instr::CallIndirect(CallIndirect {
                    ty: c.ty,
                    table: c.table,
                }),
                0x11,
                c.ty,
            ),
            _ => panic!(
                "You are not allowed to call the function `trace_call` with a non call instruction"
            ),
        };
        let typ = self.module.types.get(type_id).clone();
        let type_id: i32 = type_id.index() as i32;
        let params = self.add_locals(typ.params());
        let results = self.add_locals(typ.results());
        self.trace_code(seq, opcode);
        seq.global_get(self.mem_pointer).i32_const(type_id).store(
            self.trace_mem_id,
            ir::StoreKind::I32 { atomic: false },
            MemArg {
                offset: 1,
                align: 0,
            },
        );
        let mut offset = 5;
        params
            .into_iter()
            .map(|p| {
                let (store_kind, byte_length) = store_info(self.module.locals.get(p).ty());
                seq.global_get(self.mem_pointer).local_tee(p).store(
                    self.trace_mem_id,
                    store_kind,
                    MemArg { offset, align: 0 },
                );
                offset += byte_length;
                p
            })
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|p| {
                seq.local_get(p);
            });
        seq.instr(call);
        results
            .into_iter()
            .map(|r| {
                let (store_kind, byte_length) = store_info(self.module.locals.get(r).ty());
                seq.global_get(self.mem_pointer).local_tee(r).store(
                    self.trace_mem_id,
                    store_kind,
                    MemArg { offset, align: 0 },
                );
                offset += byte_length;
                r
            })
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|r| {
                seq.local_get(r);
            });
        self.increment_mem_pointer(seq, offset as i32);
    }

    fn trace_table_get(&mut self, seq: &mut InstrSeqBuilder, table_id: TableId) {
        let opcode = 0x25;
        self.trace_code(seq, opcode);
        let table_type = self.module.tables.get(table_id).element_ty;

        let locals = self.add_locals(&[ValType::I32, table_type]);
        seq.global_get(self.mem_pointer)
            .local_tee(*locals.get(0).unwrap())
            .store(
                self.trace_mem_id,
                StoreKind::I32 { atomic: false },
                MemArg {
                    align: 0,
                    offset: 1,
                },
            )
            .local_get(*locals.get(0).unwrap())
            .instr(TableGet { table: table_id })
            .global_get(self.mem_pointer)
            .local_set(*locals.get(0).unwrap())
            .store(
                self.trace_mem_id,
                store_info(table_type).0,
                MemArg {
                    align: 0,
                    offset: 5,
                },
            )
            .local_get(*locals.get(0).unwrap());
        self.increment_mem_pointer(seq, 5 + store_info(table_type).1 as i32);
    }
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
