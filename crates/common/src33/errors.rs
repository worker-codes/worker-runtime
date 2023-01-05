/// A convenience wrapper of `Result` that relies on
/// [`wasmtime_provider::errors::Error`](crate::errors::Error)
/// to hold errors
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// This crate's Error type
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error returned when waPC can't find one of the waPC-protocol functions.
    #[error("No such function in Wasm module")]
    NoSuchFunction(String),
    /// I/O related error.
    #[error("I/O Error: {0}")]
    IO(#[from] std::io::Error),
    /// Miscellaneous error.
    #[error("WebAssembly failure: {0}")]
    WasmMisc(String),
    /// Error during a host call.
    #[error("Error during host call: {0}")]
    HostCallFailure(Box<dyn std::error::Error + Sync + Send>),
    /// Initialization Failed.
    #[error("Initialization failed: {0}")]
    InitFailed(String),
    /// Error during a guest call.
    #[error("Guest call failure: {0}")]
    GuestCallFailure(String),
    /// Error occurred while swapping out one module for another.
    #[error("Module replacement failed: {0}")]
    ReplacementFailed(String),
    /// Error originating from a WASM Engine provider.
    #[error("WASM Provider failure: {0}")]
    ProviderFailure(Box<dyn std::error::Error + Sync + Send>),
    /// General errors.
    #[error("General: {0}")]
    General(String),

    /// Wasmtime initialization failed
    #[error("Initialization failed: {0}")]
    InitializationFailed(Box<dyn std::error::Error + Send + Sync>),

    /// Wasmtime initialization failed
    #[error("Initialization failed: {0} init interrupted, execution deadline exceeded")]
    InitializationFailedTimeout(String),

    /// The guest call function was not exported by the guest.
    #[error("Guest call function (__guest_call) not exported by wasm module.")]
    GuestCallNotFound,

    /// Error originating when wasi feature is disabled, but the user provides wasi related params
    #[error("WASI related parameter provided, but wasi feature is disabled")]
    WasiDisabled,

    /// Generic error
    // wasmtime uses `anyhow::Error` inside of its public API
    #[error(transparent)]
    Generic(#[from] anyhow::Error),
}

// impl From<Error> for wapc::errors::Error {
//     fn from(e: Error) -> Self {
//         wapc::errors::Error::ProviderFailure(Box::new(e))
//     }
// }

#[cfg(test)]
mod tests {
    #[allow(dead_code)]
    fn needs_sync_send<T: Send + Sync>() {}

    #[test]
    fn assert_sync_send() {
        needs_sync_send::<super::Error>();
    }
}
