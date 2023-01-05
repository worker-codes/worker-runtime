// use bytes::BytesMut;
// use crate::pool::{HostPool, HostPoolBuilder};
// use http::Method;
// use regress::Flags;
// use reqwest::blocking::Client;
// use reqwest::blocking::RequestBuilder;
// use reqwest::blocking::Response;
// use reqwest::Url;
// use urlpattern::UrlPattern;
// use urlpattern::UrlPatternInit;
// use urlpattern::UrlPatternMatchInput;
// use wapc_pool::HostPool;
// use wapc_pool::HostPoolBuilder;
// use core::slice::SlicePattern;
use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;
use wkr_runtime::environment::Environment;
use wkr_runtime::EnvironmentBuilder;
use wkr_runtime::errors::Error;
// use wkr_runtime::HostCallback;
// use wkr_runtime::Resource;
// use wkr_runtime::ResourceTable;
// use wkr_runtime::WapcHost;
// use wkr_runtime::WasiParams;
// use wkr_runtime::wasi::Environment;
// use std::error::Error;
// use crate::bridge::fetch::{op_fetch, FetchArgs, FetchResponse};
// use crate::bridge::crypto::{crypto_get_random_values, crypto_random_uuid, crypto_subtle_digest};
// use crate::bridge::fetch::FetchResponse;
// use crate::bridge::database::{ process_database };
// use crate::bridge::key::CryptoHash;
use anyhow::Result;
use serde::ser::StdError;
use serde::{Deserialize, Serialize};
use wkr_common::resources::{ResourceTable, Resource};
use wkr_runtime::wasi::WasiParams;
use std::fs::read;
use std::sync::Arc;
use std::sync::Mutex;
use wapc_codec::messagepack::{deserialize, serialize};
// use super::resources::{ Resource, ResourceTable };
use serde_bytes::ByteBuf;
use serde_bytes::Bytes;
// use url::Url;
// use wkr_fetch::{process_ops};

#[derive(Deserialize, Serialize, Debug)]
struct EncodeDigestArg {
    algorithm: String,
    data: ByteBuf,
    //   data: Vec<u8>,
}
#[derive(Deserialize, Serialize, Debug)]
struct GetRandomValuesArg {
    value: u32,
}



struct FetchRequestResource(String);

impl Resource for FetchRequestResource {
    fn name(&self) -> Cow<str> {
        "fetchRequest".into()
    }
}

// struct FetchClientResource(Client);

// impl Resource for FetchClientResource {
//     fn name(&self) -> Cow<str> {
//         "fetchClient".into()
//     }
// }

// struct FetchRequestBuilderResource(Arc<Mutex<RequestBuilder>>);

// impl Resource for FetchRequestBuilderResource {
//     fn name(&self) -> Cow<str> {
//         "fetchRequestBuilder".into()
//     }
// }
// struct FetchResponseResource(Response);

// impl Resource for FetchResponseResource {
//     fn name(&self) -> Cow<str> {
//         "fetchResponse".into()
//     }
// }

#[derive(Deserialize, Serialize, Debug)]
pub struct RegExpRequest {
    pub pattern: String,
    pub flag: String,
    pub input: String,
    pub last_index: i32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Match {
    pub matches: Vec<String>,
    pub index: i32,
    pub last_index: i32,
    pub input: String,
    pub indices: Vec<Vec<u32>>,
    pub groups: HashMap<String, String>,
}

pub fn create_function_engine(path: &str) -> Result<Environment> {
    // let file = read("/home/dallen/Codes/assemblyscript_test/build/release.wasm")?;
    let file = read(path)?;

    let builder = EnvironmentBuilder::new(&file);
    let engine = builder
        .wasi_params(WasiParams {
            argv: vec!["mike".to_string(), "jones".to_string()],
            map_dirs: vec![],
            env_vars: vec![("POSTGRES".to_string(), "user:password".to_string())],
            preopened_dirs: vec!["/home/dallen/Codes/wasmtest".to_string()],
        })
        .build()
        .expect("Cannot create WebAssemblyEngineProvider");

    return Ok(engine);
}

pub async fn host_callback(
    id: u64,
    binding: &str,
    namespace: &str,
    operation: &str,
    payload: &[u8],
    resource_table: Arc<Mutex<ResourceTable>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "Guest {} invoked '{}->{}:{}' with a {} byte payload",
        id,
        binding,
        namespace,
        operation,
        payload.len()
    );

    // match (binding, namespace, operation) {
    //     ("fetch", _, _) => {
    //         let result = process_ops(id, binding, namespace, operation, payload, resource_table).await;

    //         return result;
    //     }
    //     _ => {}
    // }

    // Return success with zero-byte payload.
    Ok(vec![])
}
