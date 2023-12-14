use anyhow::Result;
use walrus::{
    ir::{self, Instr, MemArg, StoreKind, Visitor, VisitorMut},
    FunctionBuilder, GlobalId, Local, LocalId, MemoryId, Module, ValType,
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
    let trace_mem_id = module.memories.add_local(true, 100, None);
    let mem_pointer = module.globals.add_local(
        walrus::ValType::I32,
        true,
        walrus::InitExpr::Value(walrus::ir::Value::I32(0)),
    );
    let mut local_collector = LocalCollector::new();
    let local_types: Vec<ValType> = module
        .funcs
        .iter_local()
        .map(|(_, func)| {
            walrus::ir::dfs_in_order(&mut local_collector, func, func.entry_block());
            local_collector.added_locals.clone()
        })
        .flatten()
        .collect();
    let local_ids: Vec<LocalId> = local_types.iter().map(|l| module.locals.add(*l)).collect();
    let locals = local_ids
        .into_iter()
        .map(|id| module.locals.get(id).clone())
        .collect();
    let builder = FunctionBuilder::new(&mut module.types, &[], &[]);
    let mut instrumenter = Instrumenter::new(trace_mem_id, mem_pointer, builder, locals);
    module.funcs.iter_local_mut().for_each(|(_, func)| {
        walrus::ir::dfs_pre_order_mut(&mut instrumenter, func, func.entry_block())
    });
    Ok(module)
}

pub struct LocalCollector {
    pub added_locals: Vec<ValType>,
}

impl Visitor<'_> for LocalCollector {
    fn visit_instr(&mut self, instr: &ir::Instr, _instr_loc: &ir::InstrLocId) {
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
            Instr::Store(store) => match store.kind {
                ir::StoreKind::I32 { .. } => {
                    self.add_fresh_locals(vec![ValType::I32, ValType::I32])
                }
                ir::StoreKind::I64 { .. } => todo!(),
                ir::StoreKind::F32 => todo!(),
                ir::StoreKind::F64 => todo!(),
                ir::StoreKind::V128 => todo!(),
                ir::StoreKind::I32_8 { .. } => todo!(),
                ir::StoreKind::I32_16 { .. } => todo!(),
                ir::StoreKind::I64_8 { .. } => todo!(),
                ir::StoreKind::I64_16 { .. } => todo!(),
                ir::StoreKind::I64_32 { .. } => todo!(),
            },
            Instr::TableGet(_) => todo!(),
            Instr::TableSet(_) => todo!(),
            Instr::TableGrow(_) => todo!(),
            Instr::TableFill(_) => todo!(),
            Instr::RefNull(_) => todo!(),
            Instr::LoadSimd(_) => todo!(),
            Instr::TableInit(_) => todo!(),
            Instr::ElemDrop(_) => todo!(),
            Instr::TableCopy(_) => todo!(),
            _ => {}
        };
    }
}

impl LocalCollector {
    pub fn new() -> Self {
        Self {
            added_locals: Vec::new(),
        }
    }
    fn add_fresh_locals(&mut self, types: Vec<ValType>) {
        let mut unused_locals = self.added_locals.clone();
        for typ in types {
            let local = match unused_locals.clone().into_iter().find(|l| *l == typ) {
                Some(typ) => typ,
                None => {
                    self.added_locals.push(typ);
                    typ
                }
            };
            if unused_locals.len() > 0 {
                let mut i = 0;
                for l in &unused_locals {
                    if *l == local {
                        break;
                    }
                    i += 1;
                }
                unused_locals.remove(i);
            }
        }
    }
}

pub struct Instrumenter {
    trace_mem_id: MemoryId,
    mem_pointer: GlobalId,
    added_locals: Vec<Local>,
    builder: FunctionBuilder,
}

impl Instrumenter {
    pub fn new(
        trace_mem_id: MemoryId,
        mem_pointer: GlobalId,
        builder: FunctionBuilder,
        added_locals: Vec<Local>,
    ) -> Self {
        Self {
            trace_mem_id,
            mem_pointer,
            added_locals,
            builder,
        }
    }

    fn local(&self, types: &[ValType]) -> Vec<LocalId> {
        let mut unused_locals = self.added_locals.clone();
        let mut output = Vec::new();
        for typ in types {
            let local = unused_locals
                .clone()
                .into_iter()
                .find(|l| l.ty() == *typ)
                .expect("This cant be there is a implementation error in the local collection");
            unused_locals = unused_locals
                .clone()
                .into_iter()
                .filter(|ul| *ul == local)
                .collect();
            output.push(local.id())
        }
        output
    }
}

impl VisitorMut for Instrumenter {
    fn visit_instr_mut(&mut self, instr: &mut ir::Instr, _instr_loc: &mut ir::InstrLocId) {
        *instr = match instr {
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
                    ir::StoreKind::I32 { .. } => (0x15, self.local(&[ValType::I32, ValType::I32])),
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
                let mut seq = self.builder.dangling_instr_seq(None);
                seq.global_get(self.mem_pointer)
                    .i32_const(opcode)
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
                    .local_get(*locals.get(1).unwrap());
                // ir::Drop {}.into()
                ir::Block { seq: seq.id() }.into()
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
            _ => instr.clone(),
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
