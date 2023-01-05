// #[macro_use]
// extern crate log;

// mod resources;
// mod server;
// // mod guest;
// mod pool;
// // mod pool2;
// mod host_pool;
// mod bridge;
// mod error;
// mod testrunner;

// use testrunner::test;
// use server::serve;

// pub fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // serve().unwrap();
//     test().unwrap();

//     Ok(())
// }
use wkr_core::run;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // serve().unwrap();
    run().await.unwrap();

    Ok(())
}
