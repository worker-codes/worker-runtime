#[macro_use]
extern crate log;

mod callbacks;
pub mod wasi;
mod environment_state;
/// The crate's error module
pub mod errors;
pub mod builder;
pub mod environment;
mod common;

pub use builder::EnvironmentBuilder;
pub use wasmtime;
pub use wasmtime_wasi;


