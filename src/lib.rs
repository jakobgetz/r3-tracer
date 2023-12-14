use anyhow::Result;
use walrus::{
    ir::{self, Instr, MemArg, StoreKind, Visitor},
    FunctionBuilder, GlobalId, InstrLocId, Local, LocalFunction, LocalId, MemoryId, Module,
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
    let module = Module::from_buffer(buffer)?;
    let mut new_module = Module::with_config(ModuleConfig::new());
    let trace_mem_id = new_module.memories.add_local(true, 100, None);
    let mem_pointer = new_module.globals.add_local(
        walrus::ValType::I32,
        true,
        walrus::InitExpr::Value(walrus::ir::Value::I32(0)),
    );
    let mut generator = Generator::new(new_module, trace_mem_id, mem_pointer);
    module
        .funcs
        .iter_local()
        .for_each(|(_, f)| generator.build(f));

    generator.module.memories = module.memories;
    generator.module.locals = module.locals;
    generator.module.tables = module.tables;
    generator.module.globals = module.globals;

    Ok(generator.module)
}

struct Generator {
    trace_mem_id: MemoryId,
    mem_pointer: GlobalId,
    added_locals: Vec<Local>,
    pub module: Module,
}

impl Generator {
    fn new(module: Module, trace_mem_id: MemoryId, mem_pointer: GlobalId) -> Self {
        Self {
            module,
            trace_mem_id,
            mem_pointer,
            added_locals: Vec::new(),
        }
    }

    fn build(&mut self, func: &LocalFunction) {
        ir::dfs_in_order(self, func, func.entry_block());
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
}

impl Visitor<'_> for Generator {
    fn visit_instr(&mut self, instr: &Instr, _instr_loc: &InstrLocId) {
        let mut builder = FunctionBuilder::new(&mut self.module.types, &[], &[]);
        let mut body = builder.func_body();
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
            Instr::Load(_) => todo!(),
            Instr::Store(store) => {
                let (opcode, locals) = match store.kind {
                    ir::StoreKind::I32 { .. } => (0x15, &[ValType::I32, ValType::I32]),
                    ir::StoreKind::I64 { .. } => todo!(),
                    ir::StoreKind::F32 => todo!(),
                    ir::StoreKind::F64 => todo!(),
                    ir::StoreKind::V128 => todo!(),
                    ir::StoreKind::I32_8 { .. } => todo!(),
                    ir::StoreKind::I32_16 { .. } => todo!(),
                    ir::StoreKind::I64_8 { .. } => todo!(),
                    ir::StoreKind::I64_16 { .. } => todo!(),
                    ir::StoreKind::I64_32 { .. } => todo!(),
                };
                let locals = self.add_fresh_locals(locals);
                body.i32_const(opcode)
                    .store(
                        self.trace_mem_id,
                        ir::StoreKind::I32_8 { atomic: false },
                        MemArg {
                            offset: 0,
                            align: 1,
                        },
                    )
                    .global_get(self.mem_pointer)
                    .local_tee(*locals.get(0).unwrap())
                    .store(
                        self.trace_mem_id,
                        ir::StoreKind::I32 { atomic: false },
                        MemArg {
                            offset: 1,
                            align: 1,
                        },
                    )
                    .global_get(self.mem_pointer)
                    .local_tee(*locals.get(1).unwrap())
                    .store(self.trace_mem_id, store.kind, store.arg)
                    .global_get(self.mem_pointer)
                    .i32_const(5 + store_byte_length(store.kind))
                    .binop(ir::BinaryOp::I32Add)
                    .global_set(self.mem_pointer)
                    .local_get(*locals.get(0).unwrap())
                    .local_get(*locals.get(1).unwrap())
                    .instr(ir::Store {
                        memory: store.memory,
                        kind: store.kind,
                        arg: store.arg,
                    });
            }
            Instr::TableGet(_) => todo!(),
            Instr::TableSet(_) => todo!(),
            Instr::TableGrow(_) => todo!(),
            Instr::TableFill(_) => todo!(),
            Instr::RefNull(_) => todo!(),
            Instr::LoadSimd(_) => todo!(),
            Instr::TableInit(_) => todo!(),
            Instr::ElemDrop(_) => todo!(),
            Instr::TableCopy(_) => todo!(),
            Instr::Block(_) => todo!(),
            Instr::Loop(_) => todo!(),
            Instr::LocalGet(_) => todo!(),
            Instr::LocalSet(_) => todo!(),
            Instr::LocalTee(_) => todo!(),
            Instr::Const(instr) => {
                body.const_(instr.value);
            }
            Instr::Binop(_) => todo!(),
            Instr::Unop(_) => todo!(),
            Instr::Select(_) => todo!(),
            Instr::Unreachable(_) => todo!(),
            Instr::Br(_) => todo!(),
            Instr::BrIf(_) => todo!(),
            Instr::IfElse(_) => todo!(),
            Instr::BrTable(_) => todo!(),
            Instr::Drop(_) => todo!(),
            Instr::MemorySize(_) => todo!(),
            Instr::AtomicRmw(_) => todo!(),
            Instr::Cmpxchg(_) => todo!(),
            Instr::AtomicNotify(_) => todo!(),
            Instr::AtomicWait(_) => todo!(),
            Instr::AtomicFence(_) => todo!(),
            Instr::TableSize(_) => todo!(),
            Instr::RefIsNull(_) => todo!(),
            Instr::RefFunc(_) => todo!(),
            Instr::V128Bitselect(_) => todo!(),
            Instr::I8x16Swizzle(_) => todo!(),
            Instr::I8x16Shuffle(_) => todo!(),
        }
    }
}

fn store_byte_length(kind: StoreKind) -> i32 {
    match kind {
        StoreKind::I32 { .. } => 4,
        StoreKind::I64 { .. } => 8,
        StoreKind::F32 => 4,
        StoreKind::F64 => 8,
        StoreKind::V128 => todo!(),
        StoreKind::I32_8 { .. } => 1,
        StoreKind::I32_16 { .. } => 2,
        StoreKind::I64_8 { .. } => 1,
        StoreKind::I64_16 { .. } => 2,
        StoreKind::I64_32 { .. } => 4,
    }
}
