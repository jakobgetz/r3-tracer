mod error;

use walrus::{
    ir::{self, Block, Instr, MemArg, VisitorMut},
    Function, FunctionBuilder, FunctionId, GlobalId, Local, LocalId, MemoryId, Module,
    ModuleLocals, ModuleTypes, ValType,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn instrument_wasm(buffer: &[u8]) -> Result<JsValue, JsValue> {
    let mut module = Module::from_buffer(buffer).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut module = InstrumentedModule::from(module);
    let mem_id = module.module.memories.add_local(true, 100, None);
    let mem_pointer = module.module.globals.add_local(
        walrus::ValType::I32,
        true,
        walrus::InitExpr::Value(walrus::ir::Value::I32(0)),
    );

    for (_, func) in module.module.funcs.iter_local_mut() {
        // let mut builder = FunctionBuilder::new(&mut module.module.types, params, results);
        // let mut builder = builder.func_body();
        for (instr, instr_id) in func.builder().func_body().instrs().iter() {
            match instr {
                Instr::Store(store) => {

                    //                 builder.instr_at(
                    //                     instr_id.data() as usize,
                    //                     ir::Const {
                    //                         value: ir::Value::I32(0),
                    //                     },
                    //                 );
                    //                 builder.instr_at(
                    //                     instr_id.data() as usize,
                    //                     ir::Store {
                    //                         memory: module.get_memory_id().unwrap(),
                    //                         kind: ir::StoreKind::I32_8 { atomic: true },
                    //                         arg: MemArg {
                    //                             align: 1,
                    //                             offset: 0,
                    //                         },
                    //                     },
                    //                 );
                }
                Instr::Load(_) => todo!(),
                Instr::TableGet(_) => todo!(),
                Instr::Call(_) => todo!(),
                Instr::Return(_) => todo!(),
                _ => {
                    // builder.instr(instr.clone());
                }
            };
            // }
            //     let builder = func.builder_mut();
            //     for (i, idx) in func.instruction_mapping.iter_mut() {
            //         builder.instr_seq(id)
        }
    }
    todo!()
    // Ok(JsValue::from(module.module.emit_wasm()))
}

struct InstrumentedModule {
    pub module: Module,
    added_locals: Vec<Local>,
}

impl From<Module> for InstrumentedModule {
    fn from(value: Module) -> Self {
        Self {
            module: value,
            added_locals: Vec::new(),
        }
    }
}

impl InstrumentedModule {
    fn add_fresh_locals(&mut self, types: Vec<ValType>) -> Vec<LocalId> {
        let mut unused_locals: Vec<Local> = self.added_locals.clone();
        types
            .into_iter()
            .map(|typ| match unused_locals.iter().find(|l| l.ty() == typ) {
                Some(local) => local.id(),
                None => self.module.locals.add(typ),
            })
            .collect::<Vec<LocalId>>()
            .into_iter()
            .map(|local_id| {
                unused_locals.iter_mut().filter(|l| l.id() != local_id);
                local_id
            })
            .collect()
    }
}

struct Instrumenter<'a> {
    builder: FunctionBuilder,
    trace_mem_id: MemoryId,
    mem_pointer: GlobalId,
    module: &'a InstrumentedModule,
}

impl<'a> VisitorMut for Instrumenter<'a> {
    fn visit_store_mut(&mut self, store: &mut ir::Store) {
        let (opcode, locals) = match store.kind {
            ir::StoreKind::I32 { .. } => (0x15, vec![ValType::I32, ValType::I32]),
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
        let local_ids = self.module.add_fresh_locals(locals);
        let mut seq = self.builder.dangling_instr_seq(None);
        let seq_id = seq
            .global_get(self.mem_pointer)
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
            .local_tee(*local_ids.get(0).unwrap())
            .id();
        *store = ir::Block { seq: seq_id }
    }
}
