mod fetch;

use anyhow::Error;
use fetch::{FetchRequest, op_fetch};
use wkr_common::resources::ResourceTable;
use std::sync::Arc;
use tokio::sync::Mutex;
use wapc_codec::messagepack::{deserialize, serialize};

pub async fn process_ops(
    _id: u64,
    binding: &str,
    namespace: &str,
    operation: &str,
    payload: &[u8],
    resource_table: Arc<Mutex<ResourceTable>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    match (binding, namespace, operation) {
        ("fetch", "init", _) => {
            let state = resource_table.lock().await;
            let fetch_args: FetchRequest = deserialize(payload).unwrap();
            let resp = op_fetch(state, fetch_args).await.unwrap();
            let fetch_response = serialize(&resp).unwrap();

            return Ok(fetch_response);
        }
        ("fetch", "send", _) => {
            let state = resource_table.lock().await;
            let fetch_args: FetchRequest = deserialize(payload).unwrap();
            let resp = op_fetch(state, fetch_args).await.unwrap();
            let fetch_response = serialize(&resp).unwrap();

            return Ok(fetch_response);
        }
        ("fetch", "read_body", _) => {
            let state = resource_table.lock().await;
            let fetch_args: FetchRequest = deserialize(payload).unwrap();
            let resp = op_fetch(state, fetch_args).await.unwrap();
            let fetch_response = serialize(&resp).unwrap();

            return Ok(fetch_response);
        }
        ("fetch", "write_body", _) => {
            let state = resource_table.lock().await;
            let fetch_args: FetchRequest = deserialize(payload).unwrap();
            let resp = op_fetch(state, fetch_args).await.unwrap();
            let fetch_response = serialize(&resp).unwrap();

            return Ok(fetch_response);
        }
        _ => {}
    }

    Ok(vec![])
}
