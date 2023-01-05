use crate::errors::Result;
// use crate::Environment;
use crate::wasi::WasiParams;
use crate::environment::{EpochDeadlines, Environment};


#[derive(Default)]
pub struct EnvironmentBuilder<'a> {
  module_bytes: &'a [u8],
  wasi_params: Option<WasiParams>,
  epoch_deadlines: Option<EpochDeadlines>,
}

impl<'a> EnvironmentBuilder<'a> {
  /// A new EnvironmentBuilder instance,
  /// must provide the wasm module to be loaded
  #[must_use]
  pub fn new(module_bytes: &'a [u8]) -> Self {
    EnvironmentBuilder {
      module_bytes,
      ..Default::default()
    }
  }
  /// WASI params
  #[must_use]
  pub fn wasi_params(mut self, wasi: WasiParams) -> Self {
    self.wasi_params = Some(wasi);
    self
  }


  /// Enable Wasmtime [epoch-based interruptions](wasmtime::Config::epoch_interruption) and set
  /// the deadlines to be enforced
  ///
  /// Two kind of deadlines have to be set:
  ///
  /// * `wapc_init_deadline`: the number of ticks the waPC initialization code can take before the
  ///   code is interrupted. This is the code usually defined inside of the `wapc_init`/`_start`
  ///   functions
  /// * `wapc_func_deadline`: the number of ticks any regular waPC guest function can run before
  ///   its terminated by the host
  ///
  /// Both these limits are expressed using the number of ticks that are allowed before the
  /// WebAssembly execution is interrupted.
  /// It's up to the embedder of waPC to define how much time a single tick is granted. This could
  /// be 1 second, 10 nanoseconds, or whatever the user prefers.
  ///
  /// **Warning:** when providing an instance of `wasmtime::Engine` via the
  /// `Environment::engine` helper, ensure the `wasmtime::Engine`
  /// has been created with the `epoch_interruption` feature enabled
  #[must_use]
  pub fn enable_epoch_interruptions(mut self, wapc_init_deadline: u64, wapc_func_deadline: u64) -> Self {
    self.epoch_deadlines = Some(EpochDeadlines {
      wapc_init: wapc_init_deadline,
      wapc_func: wapc_func_deadline,
    });
    self
  }

  /// Create a `Environment` instance
  pub fn build(&self) -> Result<Environment> {
    
    let mut config = wasmtime::Config::default();
    config.async_support(true);
    config.consume_fuel(true);
    
    if self.epoch_deadlines.is_some() {
      config.epoch_interruption(true);
    }

    let engine = wasmtime::Engine::new(&config)?;
    let mut provider = Environment::new_with_engine(self.module_bytes, engine, self.wasi_params.clone())?;
    provider.epoch_deadlines = self.epoch_deadlines;

    Ok(provider)
  }
}
