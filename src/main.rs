extern crate r3_tracer;
use std::fs;

use r3_tracer::instrument_wasm;

fn main() {
    let module = instrument_wasm(&fs::read("./test.wasm").unwrap());
    let _ = dbg!(module);
}
