mod error;

use walrus::{
    ir::{self, Block, Instr, MemArg},
    MemoryId,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn instrument_wasm(buffer: &[u8]) -> Result<JsValue, JsValue> {
    let mut module =
        walrus::Module::from_buffer(buffer).map_err(|e| JsValue::from_str(&e.to_string()))?;
    module.memories.add_local(true, 100, None);
    let mem_pointer = module.globals.add_local(
        walrus::ValType::I32,
        true,
        walrus::InitExpr::Value(walrus::ir::Value::I32(0)),
    );

    for (_, func) in module.funcs.iter_local_mut() {
        let builder = func.builder().func_body();
        let entry_id = func.entry_block();
        for (instr, instr_id) in builder.instrs().iter() {
            match instr {
                Instr::Store(_) => {
                    builder.instr_at(
                        instr_id.data() as usize,
                        ir::GlobalGet {
                            global: mem_pointer,
                        },
                    );
                    builder.instr_at(
                        instr_id.data() as usize,
                        ir::Const {
                            value: ir::Value::I32(0),
                        },
                    );
                    builder.instr_at(
                        instr_id.data() as usize,
                        ir::Store {
                            memory: module.get_memory_id().unwrap(),
                            kind: ir::StoreKind::I32_8 { atomic: true },
                            arg: MemArg {
                                align: 1,
                                offset: 0,
                            },
                        },
                    );
                }
                Instr::Load(_) => todo!(),
                Instr::TableGet(_) => todo!(),
                Instr::Call(_) => todo!(),
                Instr::Return(_) => todo!(),
            };
        }
        let builder = func.builder_mut();
        for (i, idx) in func.instruction_mapping.iter_mut() {
            builder.instr_seq(id)
        }
    }
    todo!()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
