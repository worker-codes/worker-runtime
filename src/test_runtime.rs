use wkr_core::run;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // serve().unwrap();
    run().await.unwrap();

    Ok(())
}
