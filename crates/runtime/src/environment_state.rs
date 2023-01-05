use std::{sync::{Arc}};
use parking_lot::RwLock;
use wkr_common::resources::ResourceTable;
use wasmtime_wasi::WasiCtx;
use wkr_fetch::process_ops;
use wkr_database::process_database_ops;
use tokio::sync::Mutex;
use crate::common::Invocation;

/// Module state is essentially a 'handle' that is passed to a runtime engine to allow it
/// to read and write relevant data as different low-level functions are executed during
/// a waPC conversation
pub struct EnvironmentState {  
  pub wasi_ctx: WasiCtx,

  pub guest_request: Arc<RwLock<Option<Invocation>>>,
  pub guest_response: Arc<RwLock<Option<Vec<u8>>>>,
  pub host_response: Arc<RwLock<Option<Vec<u8>>>>,
  pub guest_error: Arc<RwLock<Option<String>>>,
  pub host_error: Arc<RwLock<Option<String>>>,
  // pub host_callback: Option<Box<HostCallback>>,
  pub id: u64,
  pub resource_table: Arc<Mutex<ResourceTable>>,
}

impl EnvironmentState {
    /// Retrieves the value, if any, of the current guest request
    pub fn get_guest_request(&self) -> Option<Invocation> {
      self.guest_request.read().clone()
    }

    pub fn get_guest_error(&self) -> Option<String> {
      self.guest_error.read().clone()
    }
  
    /// Retrieves the value of the current host response
    pub fn get_host_response(&self) -> Option<Vec<u8>> {
      self.host_response.read().clone()
    }
  
    pub fn set_host_response(&self, buf: Vec<u8>) {
      *self.host_response.write() = Some(buf);
    }
  
  
    /// Sets a value indicating that an error occurred inside the execution of a guest call
    pub fn set_guest_error(&self, error: String) {
      *self.guest_error.write() = Some(error);
    }
  
    /// Sets the value indicating the response data from a guest call
    pub fn set_guest_response(&self, response: Vec<u8>) {
      *self.guest_response.write() = Some(response);
    }
  
    /// Queries the value of the current guest response
    pub fn get_guest_response(&self) -> Option<Vec<u8>> {
      self.guest_response.read().clone()
    }
  
    /// Queries the value of the current host error
    pub fn get_host_error(&self) -> Option<String> {
      self.host_error.read().clone()
    }
  
    pub fn set_host_error(&self, error: String) {
      *self.host_error.write() = Some(error);
    }
      /// Invoked when the guest module wants to write a message to the host's `stdout`
  pub fn do_console_log(&self, msg: &str) {
    info!("Guest module {}: {}", self.id, msg);
  }
  pub async fn do_host_call(
    &self,
    id: u64,
    binding: &str,
    namespace: &str,
    operation: &str,
    payload: &[u8],
    resource_table: Arc<Mutex<ResourceTable>>,
  // ) -> Result<Vec<u8>> {
  ) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    
    match (binding, namespace, operation) {
        ("fetch", _, _) => {
          println!("fetch>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
            let result = process_ops(id, binding, namespace, operation, payload, resource_table).await;

            return result;
        },
        ("database", _, _) =>{
          println!("database>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
          let result = process_database_ops(id, binding, namespace, operation, payload, resource_table).await;

            return result;
        }
        _ => {}
    }

      Ok(vec![])
  }
}

impl std::fmt::Debug for EnvironmentState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("EnvironmentState")
      .field("guest_request", &self.guest_request)
      .field("guest_response", &self.guest_response)
      .field("host_response", &self.host_response)
      .field("guest_error", &self.guest_error)
      .field("host_error", &self.host_error)
      // .field("host_callback", &self.host_callback.as_ref().map(|_| Some("Some(Fn)")))
      .field("id", &self.id)
      .finish()
  }
}