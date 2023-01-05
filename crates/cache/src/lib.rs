use std::fs::read;

use wapc::{errors, WapcHost};
use wapc_codec::messagepack::{deserialize, serialize};

pub fn test() -> Result<(), errors::Error> {
    let buf = read("/home/dallen/Codes/assemblyscript/testing/builds/myModule.wasm")?;

    let engine = wasmtime_provider::WasmtimeEngineProvider::new(&buf, None)?;
    let guest = WapcHost::new(
        Box::new(engine),
        Some(Box::new(move |_a, _b, _c, _d, _e,rt| Ok(vec![]))),
    )?;

    let callresult = guest.call("test", &serialize("hello world").unwrap())?;
    // let result: String = deserialize(&callresult).unwrap();
    // conver to String
    let result = String::from_utf8(callresult).unwrap();
    // assert_eq!(result, "hello world");
    Ok(())
}