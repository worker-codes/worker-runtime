use wkr_core::{create_function_engine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = "/home/dallen/WorkerCodes/WASM/worker-script/testing/builds/myModule.wasm";
    let mut environment = create_function_engine(file).await?;
    environment.init().await?;
    let guest_result = environment.call("test", &vec![]).await?;

    let _result = String::from_utf8(guest_result).unwrap();
    Ok(())
}
    // 