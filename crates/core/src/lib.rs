
use tokio::fs::read;
use wapc_codec::messagepack::serialize;
use anyhow::Result;
use wkr_runtime::{environment::Environment, EnvironmentBuilder, wasi::WasiParams};

pub async fn run() -> Result<()> {
    let mut environment = create_function_engine("/home/dallen/Codes/assemblyscript/testing/builds/myModule.wasm").await.unwrap();
    environment.init().await?;
    // let host = create_function_pool(engine).unwrap();
    // let store = engine.store;
    // let engine = wasmtime_provider::WasmtimeEngineProvider::new(&buf, None)?;
    // let guest = WapcHost::new(
    //     Box::new(engine),
    //     // Some(Box::new(host_callback)),
    //     None,
    // ).await.unwrap();

    
    let callresult = environment.call("test", &serialize("hello world").unwrap()).await.unwrap();
    // let result: String = deserialize(&callresult).unwrap();
    // conver to String
    let result = String::from_utf8(callresult).unwrap();
    // assert_eq!(result, "hello world");
    Ok(())
}

pub async fn create_function_engine(path: &str) -> Result<Environment> {
    // let file = read("/home/dallen/Codes/assemblyscript_test/build/release.wasm")?;
    let file = read(path).await?;

    let builder = EnvironmentBuilder::new(&file);
    let engine = builder
        .wasi_params(WasiParams {
            argv: vec!["mike".to_string(), "jones".to_string()],
            map_dirs: vec![
                // ("mike".to_string(), "/home/dallen/Codes/wasmtest/wasi/".to_string()),
                // ("mike2".to_string(), "/home/dallen/Codes/wasmtest/wasi/folder2/".to_string()),
                ],
            // map_dirs: vec![],
            env_vars: vec![("POSTGRES".to_string(), "user:password".to_string())],
            // preopened_dirs: vec!["/home/dallen/Codes/wasmtest/wasi".to_string()],
            preopened_dirs: vec![],
        })
        .build()
        .expect("Cannot create WebAssemblyEngineProvider");

    return Ok(engine);
}

pub async fn create_function_engine_with_bytes(module: Vec<u8>) -> Result<Environment> {

    let builder = EnvironmentBuilder::new(&module);
    let engine = builder
        .wasi_params(WasiParams {
            argv: vec!["mike".to_string(), "jones".to_string()],
            map_dirs: vec![
                // ("mike".to_string(), "/home/dallen/Codes/wasmtest/wasi/".to_string()),
                // ("mike2".to_string(), "/home/dallen/Codes/wasmtest/wasi/folder2/".to_string()),
                ],
            // map_dirs: vec![],
            env_vars: vec![("POSTGRES".to_string(), "user:password".to_string())],
            preopened_dirs: vec!["/home/dallen/Codes/wasmtest/wasi".to_string()],
            // preopened_dirs: vec![],
        })
        .build()
        .expect("Cannot create WebAssemblyEngineProvider");

    return Ok(engine);
}