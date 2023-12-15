extern crate r3_tracer;
use std::fs;

use r3_tracer::instrument_wasm;

fn main() {
    let mut module = instrument_wasm(&fs::read("./test.wasm").unwrap()).unwrap();
    // let _ = dbg!(&module);
    let _ = module.emit_wasm_file("./instrumented.wasm");
}
