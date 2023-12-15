use anyhow::Result;
use walrus::{
    ir::{self, Instr, MemArg, StoreKind},
    FunctionBuilder, GlobalId, InstrSeqBuilder, Local, LocalFunction, LocalId, MemoryId, Module,
    ModuleConfig, ValType,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn instrument_wasm_js(buffer: &[u8]) -> Result<JsValue, JsValue> {
    let _ = instrument_wasm(buffer).map_err(|e| JsValue::from_str(&e.to_string()));
    todo!()
    // Ok(JsValue::from(module.module.emit_wasm()))
}

pub fn instrument_wasm(buffer: &[u8]) -> Result<Module> {
    let mut module = Module::from_buffer(buffer)?;
    let mut new_module = Module::with_config(ModuleConfig::new());
    new_module.memories = module.memories;
    new_module.locals = module.locals;
    new_module.tables = module.tables;
    new_module.globals = module.globals;
    let mut generator = Generator::new(new_module);
    module
        .funcs
        .iter_local_mut()
        .for_each(|(_, f)| generator.build(f));

    Ok(generator.module)
}

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
        let mut new_body = builder.func_body();
        let body = func.builder_mut().func_body();
        let instrs = body.instrs();
        for (instr_ref, _) in instrs {
            let instr = instr_ref.clone();
            match instr {
                Instr::Call(_) => todo!(),
                Instr::CallIndirect(_) => todo!(),
                Instr::GlobalGet(_) => todo!(),
                Instr::GlobalSet(_) => todo!(),
                Instr::Return(_) => todo!(),
                Instr::MemoryGrow(_) => todo!(),
                Instr::MemoryInit(_) => todo!(),
                Instr::DataDrop(_) => todo!(),
                Instr::MemoryCopy(_) => todo!(),
                Instr::MemoryFill(_) => todo!(),
                Instr::Load(load) => {
                    let (opcode, locals, byte_length) = match load.kind {
                        ir::LoadKind::I32 { .. } => (0x28, &[ValType::I32], 4),
                        ir::LoadKind::I64 { .. } => (0x29, &[ValType::I32], 8),
                        ir::LoadKind::F32 => (0x2A, &[ValType::I32], 4),
                        ir::LoadKind::F64 => (0x2B, &[ValType::I32], 8),
                        ir::LoadKind::V128 => todo!(),
                        ir::LoadKind::I32_8 { .. } => (0x2C, &[ValType::I32], 1),
                        ir::LoadKind::I32_16 { .. } => (0x2E, &[ValType::I32], 2),
                        ir::LoadKind::I64_8 { .. } => (0x30, &[ValType::I32], 1),
                        ir::LoadKind::I64_16 { .. } => (0x32, &[ValType::I32], 2),
                        ir::LoadKind::I64_32 { .. } => (0x34, &[ValType::I32], 4),
                    };
                    let locals = self.add_fresh_locals(locals);
                    new_body
                        .global_get(self.mem_pointer)
                        .i32_const(opcode)
                        .store(
                            self.trace_mem_id,
                            ir::StoreKind::I32_8 { atomic: false },
                            MemArg {
                                offset: 0,
                                align: 0,
                            },
                        )
                        .global_get(self.mem_pointer)
                        .local_tee(*locals.get(0).unwrap())
                        .store(
                            self.trace_mem_id,
                            to_store_kind(byte_length),
                            MemArg {
                                offset: 1,
                                align: 0,
                            },
                        )
                        .global_get(self.mem_pointer)
                        .local_tee(*locals.get(1).unwrap())
                        .store(
                            self.trace_mem_id,
                            ir::StoreKind::I32 { atomic: false },
                            MemArg {
                                offset: 1 + byte_length,
                                align: 0,
                            },
                        );
                    self.increment_mem_pointer(&mut new_body, 5 + byte_length as i32);
                    new_body
                        .local_get(*locals.get(1).unwrap())
                        .local_get(*locals.get(0).unwrap())
                        .instr(load);
                }
                Instr::Store(store) => {
                    let (opcode, locals, byte_length) = match store.kind {
                        ir::StoreKind::I32 { .. } => (0x36, &[ValType::I32, ValType::I32], 4),
                        ir::StoreKind::I64 { .. } => (0x37, &[ValType::I32, ValType::I64], 8),
                        ir::StoreKind::F32 => (0x38, &[ValType::I32, ValType::F32], 4),
                        ir::StoreKind::F64 => (0x39, &[ValType::I32, ValType::F64], 8),
                        ir::StoreKind::V128 => todo!(),
                        ir::StoreKind::I32_8 { .. } => (0x3A, &[ValType::I32, ValType::I32], 1),
                        ir::StoreKind::I32_16 { .. } => (0x3B, &[ValType::I32, ValType::I32], 2),
                        ir::StoreKind::I64_8 { .. } => (0x3C, &[ValType::I32, ValType::I64], 1),
                        ir::StoreKind::I64_16 { .. } => (0x3D, &[ValType::I32, ValType::I64], 2),
                        ir::StoreKind::I64_32 { .. } => (0x3E, &[ValType::I32, ValType::I64], 4),
                    };
                    let locals = self.add_fresh_locals(locals);
                    new_body
                        .global_get(self.mem_pointer)
                        .i32_const(opcode)
                        .store(
                            self.trace_mem_id,
                            ir::StoreKind::I32_8 { atomic: false },
                            MemArg {
                                offset: 0,
                                align: 0,
                            },
                        )
                        .global_get(self.mem_pointer)
                        .local_tee(*locals.get(0).unwrap())
                        .store(
                            self.trace_mem_id,
                            store.kind,
                            MemArg {
                                offset: 1,
                                align: 0,
                            },
                        )
                        .global_get(self.mem_pointer)
                        .local_tee(*locals.get(1).unwrap())
                        .store(
                            self.trace_mem_id,
                            ir::StoreKind::I32 { atomic: false },
                            MemArg {
                                offset: 1 + byte_length,
                                align: 0,
                            },
                        );
                    self.increment_mem_pointer(&mut new_body, 5 + byte_length as i32);
                    new_body
                        .local_get(*locals.get(1).unwrap())
                        .local_get(*locals.get(0).unwrap())
                        .instr(store);
                }
                Instr::TableGet(_) => todo!(),
                Instr::TableSet(_) => todo!(),
                Instr::TableGrow(_) => todo!(),
                Instr::TableFill(_) => todo!(),
                Instr::LoadSimd(_) => todo!(),
                Instr::TableInit(_) => todo!(),
                Instr::ElemDrop(_) => todo!(),
                Instr::TableCopy(_) => todo!(),
                _ => {
                    new_body.instr(instr);
                }
            }
        }
        builder.finish(func.args.clone(), &mut self.module.funcs);
    }

    fn add_fresh_locals(&mut self, types: &[ValType]) -> Vec<LocalId> {
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

    fn increment_mem_pointer(&self, seq: &mut InstrSeqBuilder, amount: i32) {
        seq.global_get(self.mem_pointer)
            .i32_const(amount)
            .binop(ir::BinaryOp::I32Add)
            .global_set(self.mem_pointer);
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
