extern crate r3_tracer;
use std::fs;

use r3_tracer::instrument_wasm;

fn main() {
    let test_name = "call";
    let buffer = &fs::read(format!("../tests/{}.wasm", test_name)).unwrap();
    let mut module = instrument_wasm(buffer).unwrap();
    // let _ = dbg!(&module);
    let _ = module.emit_wasm_file(format!("../tests/{}-instrumented.wasm", test_name));
}
