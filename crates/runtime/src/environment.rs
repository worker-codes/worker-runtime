use crate::common::{Invocation, abi};
use crate::errors::{Error, Result, self};
use crate::environment_state::EnvironmentState;
use crate::wasi::{self, WasiParams};
use crate::{callbacks};
use parking_lot::RwLock;
use std::sync::{Arc};
use tokio::sync::Mutex;
use wasmtime::{
    AsContextMut, Engine, Extern, ExternType, Instance, Linker, Module, Store, TypedFunc,
};
use wasmtime_wasi::WasiCtx;
use wkr_common::resources::ResourceTable;

/// The host module name / namespace that guest modules must use for imports
pub const HOST_NAMESPACE: &str = "wapc";
// namespace needed for some language support
const WASI_UNSTABLE_NAMESPACE: &str = "wasi_unstable";
const WASI_SNAPSHOT_PREVIEW1_NAMESPACE: &str = "wasi_snapshot_preview1";

struct EngineInner {
    instance: Arc<RwLock<Instance>>,
    guest_call_fn: TypedFunc<(i32, i32), i32>,
}

/// Configure behavior of wasmtime [epoch-based interruptions](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption)
///
/// There are two kind of deadlines that apply to waPC modules:
///
/// * waPC initialization code: this is the code defined by the module inside
///   of the `wapc_init` or the `_start` functions
/// * user function: the actual waPC guest function written by an user
#[derive(Clone, Copy, Debug)]
pub struct EpochDeadlines {
    /// Deadline for waPC initialization code. Expressed in number of epoch ticks
    pub wapc_init: u64,

    /// Deadline for user-defined waPC function computation. Expressed in number of epoch ticks
    pub wapc_func: u64,
}

/// A waPC engine provider that encapsulates the Wasmtime WebAssembly runtime
#[allow(missing_debug_implementations)]
pub struct Environment {
    module: Module,
    wasi_params: WasiParams,
    inner: Option<EngineInner>,
    pub store: Store<EnvironmentState>,
    engine: Engine,
    linker: Linker<EnvironmentState>,
    pub epoch_deadlines: Option<EpochDeadlines>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        let engine = self.engine.clone();
        let wasi_ctx = init_wasi(&self.wasi_params).unwrap();
        let resource_table = Arc::new(Mutex::new(ResourceTable::default()));
        let store = Store::new(
            &engine,
            EnvironmentState {
                wasi_ctx,
                id: 0,
                guest_request: Arc::new(RwLock::new(None)),
                guest_response: Arc::new(RwLock::new(None)),
                host_response: Arc::new(RwLock::new(None)),
                guest_error: Arc::new(RwLock::new(None)),
                host_error: Arc::new(RwLock::new(None)),
                resource_table,
            },
        );

        match &self.inner {
            Some(_state) => {
                let new = Self {
                    module: self.module.clone(),
                    inner: None,
                    store,
                    engine,
                    epoch_deadlines: self.epoch_deadlines,
                    linker: self.linker.clone(),
                    wasi_params: self.wasi_params.clone(),
                };
                // new.init();
                new
            }
            None => Self {
                module: self.module.clone(),
                inner: None,
                store,
                engine,
                epoch_deadlines: self.epoch_deadlines,
                linker: self.linker.clone(),
                wasi_params: self.wasi_params.clone(),
            },
        }
    }
}

impl Environment {
    pub fn new_with_engine(buf: &[u8], engine: Engine, wasi: Option<WasiParams>) -> Result<Self> {
        let module = Module::new(&engine, buf)?;

        let mut linker: Linker<EnvironmentState> = Linker::new(&engine);
        wasmtime_wasi::tokio::add_to_linker(&mut linker, |s| &mut s.wasi_ctx).unwrap();
        let wasi_params = wasi.unwrap_or_default();
        let wasi_ctx = wasi::init_ctx(
            &wasi::compute_preopen_dirs(&wasi_params.preopened_dirs, &wasi_params.map_dirs)
                .unwrap(),
            &wasi_params.argv,
            &wasi_params.env_vars,
        )
        .unwrap();
        let resource_table = Arc::new(Mutex::new(ResourceTable::default()));
        let mut store = Store::new(
            &engine,
            EnvironmentState {
                wasi_ctx,
                id: 0,
                guest_request: Arc::new(RwLock::new(None)),
                guest_response: Arc::new(RwLock::new(None)),
                host_response: Arc::new(RwLock::new(None)),
                guest_error: Arc::new(RwLock::new(None)),
                host_error: Arc::new(RwLock::new(None)),
                resource_table,
            },
        );
        store.out_of_fuel_async_yield(u64::MAX, 10000);

        Ok(Environment {
            module,
            // #[cfg(feature = "wasi")]
            wasi_params,
            inner: None,
            store,
            engine,
            linker,
            epoch_deadlines: None,
        })
    }

    pub async fn init(
        &mut self,
    ) -> Result<()> {
        let instance = instance_from_module(&mut self.store, &self.module, &self.linker).await?;
        let instance_ref = Arc::new(RwLock::new(instance));
        let gc = guest_call_fn(self.store.as_context_mut(), &instance_ref)?;
        self.inner = Some(EngineInner {
            instance: instance_ref,
            guest_call_fn: gc,
        });
        self.initialize().await?;
        Ok(())
    }

    fn set_store(&mut self, inv: Invocation) {
        let store = self.store.data_mut();

        {
            *store.guest_response.write() = None;
            *store.guest_request.write() = Some(inv);
            *store.guest_error.write() = None;
            *store.host_response.write() = None;
            *store.host_error.write() = None;
        }
    }

    fn get_guest_error(&mut self) -> Option<String> {
        let store = self.store.data();
        store.guest_error.read().clone()
    }

    fn get_guest_response(&mut self) -> Option<Vec<u8>> {
        let store = self.store.data();
        store.guest_response.read().clone()
    }
    pub async fn call(&mut self, op: &str, payload: &[u8]) -> Result<Vec<u8>> {
        let inv = Invocation::new(op, payload.to_vec());
        let op_len = inv.operation.len();
        let msg_len = inv.msg.len();

        self.set_store(inv);

        let callresult = match self.call_engine(op_len as i32, msg_len as i32).await {
            Ok(c) => c,
            Err(e) => {
                return Err(errors::Error::GuestCallFailure(e.to_string()));
            }
        };

        if callresult == 0 {
            // invocation failed
            // let lock = store.guest_error.read();
            let lock = self.get_guest_error();
            match lock {
                Some(ref s) => Err(errors::Error::GuestCallFailure(s.clone())),
                None => Err(errors::Error::GuestCallFailure(
                    "No error message set for call failure".to_owned(),
                )),
            }
        } else {
            // invocation succeeded
            match self.get_guest_response() {
                Some(ref e) => Ok(e.clone()),
                None => {
                    // let lock = store.guest_error.read();
                    let lock = self.get_guest_error();
                    match lock {
                        Some(ref s) => Err(errors::Error::GuestCallFailure(s.clone())),
                        None => Err(errors::Error::GuestCallFailure(
                            "No error message OR response set for call success".to_owned(),
                        )),
                    }
                }
            }
        }
    }

    async fn call_engine(
        &mut self,
        op_length: i32,
        msg_length: i32,
    ) -> std::result::Result<i32, Box<(dyn std::error::Error + Send + Sync + 'static)>> {
        if let Some(deadlines) = &self.epoch_deadlines {
            // the deadline counter must be set before invoking the wasm function
            self.store.set_epoch_deadline(deadlines.wapc_func);
        }

        let engine_inner = self.inner.as_ref().unwrap();
        let call = engine_inner
            .guest_call_fn
            .call_async(&mut self.store, (op_length, msg_length))
            .await;

        match call {
            Ok(result) => Ok(result),
            Err(trap) => {
                error!("Failure invoking guest module handler: {:?}", trap);
                let guest_error = trap.to_string();
                // if let Some(trap_code) = trap.trap_code() {
                //   if matches!(trap_code, wasmtime::TrapCode::Interrupt) {
                //     guest_error = "guest code interrupted, execution deadline exceeded".to_owned();
                //   }
                // }
                // let mut host = engine_inner.host.lock().unwrap();
                let store = self.store.data();
                store.set_guest_error(guest_error);
                Ok(0)
            }
        }
    }

    async fn replace(
        &mut self,
        module: &[u8],
    ) -> std::result::Result<(), Box<(dyn std::error::Error + Send + Sync + 'static)>> {
        info!(
            "HOT SWAP - Replacing existing WebAssembly module with new buffer, {} bytes",
            module.len()
        );

        let new_instance =
            instance_from_buffer(&mut self.store, &self.engine, module, &self.linker).await?;
        *self.inner.as_ref().unwrap().instance.write() = new_instance;

        Ok(self.initialize().await?)
    }
    async fn initialize(&mut self) -> Result<()> {
        for starter in abi::REQUIRED_STARTS.iter() {
            if let Some(deadlines) = &self.epoch_deadlines {
                // the deadline counter must be set before invoking the wasm function
                self.store.set_epoch_deadline(deadlines.wapc_init);
            }

            let engine_inner = self.inner.as_ref().unwrap();
            if engine_inner
                .instance
                .read()
                .get_export(&mut self.store, starter)
                .is_some()
            {
                // Need to get a `wasmtime::TypedFunc` because its `call` method
                // can return a Trap error. Non-typed functions instead return a
                // generic `anyhow::Error` that doesn't allow nice handling of
                // errors
                let starter_func: TypedFunc<(), ()> = engine_inner
                    .instance
                    .read()
                    .get_typed_func(&mut self.store, starter)?;
                starter_func
                    .call_async(&mut self.store, ())
                    .await
                    .map_err(|trap| {
                        Error::InitializationFailed(trap.into())
                        // if let Some(trap_code) = trap.trap_code() {
                        //   if matches!(trap_code, wasmtime::TrapCode::Interrupt) {
                        //     Error::InitializationFailedTimeout((*starter).to_owned())
                        //   } else {
                        //     Error::InitializationFailed(trap.into())
                        //   }
                        // } else {
                        //   Error::InitializationFailed(trap.into())
                        // }
                    })?;
            }
        }
        Ok(())
    }
}

async fn instance_from_buffer(
    store: &mut Store<EnvironmentState>,
    engine: &Engine,
    buf: &[u8],
    linker: &Linker<EnvironmentState>,
) -> Result<Instance> {
    let module = Module::new(engine, buf).unwrap();
    let imports = arrange_imports(&module, store, linker);
    Ok(
        wasmtime::Instance::new_async(store.as_context_mut(), &module, imports?.as_slice())
            .await
            .unwrap(),
    )
}

async fn instance_from_module(
    store: &mut Store<EnvironmentState>,
    module: &Module,
    linker: &Linker<EnvironmentState>,
) -> Result<Instance> {
    let imports = arrange_imports(module, store, linker);
    Ok(
        wasmtime::Instance::new_async(store.as_context_mut(), module, imports?.as_slice())
            .await
            .unwrap(),
    )
}

// #[cfg(feature = "wasi")]
fn init_wasi(params: &WasiParams) -> Result<WasiCtx> {
    wasi::init_ctx(
        &wasi::compute_preopen_dirs(&params.preopened_dirs, &params.map_dirs).unwrap(),
        &params.argv,
        &params.env_vars,
    )
    .map_err(|e| Error::InitializationFailed(e))
}

/// wasmtime requires that the list of callbacks be "zippable" with the list
/// of module imports. In order to ensure that both lists are in the same
/// order, we have to loop through the module imports and instantiate the
/// corresponding callback. We **cannot** rely on a predictable import order
/// in the wasm module
#[allow(clippy::unnecessary_wraps)]
fn arrange_imports(
    module: &Module,
    store: &mut Store<EnvironmentState>,
    linker: &Linker<EnvironmentState>,
) -> Result<Vec<Extern>> {
    Ok(module
        .imports()
        .filter_map(|imp| {
            if let ExternType::Func(_) = imp.ty() {
                match imp.module() {
                    HOST_NAMESPACE => Some(callback_for_import(store, imp.name())),
                    WASI_SNAPSHOT_PREVIEW1_NAMESPACE | WASI_UNSTABLE_NAMESPACE => {
                        linker.get_by_import(store.as_context_mut(), &imp)
                    }
                    other => panic!("import module `{}` was not found", other), //TODO: get rid of panic
                }
            } else {
                None
            }
        })
        .collect())
}

fn callback_for_import(store: &mut Store<EnvironmentState>, import: &str) -> Extern {
    match import {
        abi::HOST_CONSOLE_LOG => callbacks::console_log_func(store).into(),
        abi::HOST_CALL => callbacks::host_call_func(store).into(),
        abi::GUEST_REQUEST_FN => callbacks::guest_request_func(store).into(),
        abi::HOST_RESPONSE_FN => callbacks::host_response_func(store).into(),
        abi::HOST_RESPONSE_LEN_FN => callbacks::host_response_len_func(store).into(),
        abi::GUEST_RESPONSE_FN => callbacks::guest_response_func(store).into(),
        abi::GUEST_ERROR_FN => callbacks::guest_error_func(store).into(),
        abi::HOST_ERROR_FN => callbacks::host_error_func(store).into(),
        abi::HOST_ERROR_LEN_FN => callbacks::host_error_len_func(store).into(),
        _ => unreachable!(),
    }
}

// Called once, then the result is cached. This returns a `Func` that corresponds
// to the `__guest_call` export
fn guest_call_fn(
    store: impl AsContextMut,
    instance: &Arc<RwLock<Instance>>,
) -> Result<TypedFunc<(i32, i32), i32>> {
    instance
        .read()
        .get_typed_func::<(i32, i32), i32>(store, abi::GUEST_CALL)
        .map_err(|_| Error::GuestCallNotFound)
}
