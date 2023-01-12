mod fetch;

use anyhow::Error;
use fetch::{FetchRequest, op_fetch, op_fetch_send, op_fetch_read_body, FetchReadBodyReturn, FetchReadBody, FetchWriteBody, op_fetch_write_body};
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
            println!("fetch_args: {:?}", fetch_args );
            let resp = op_fetch(state, fetch_args).await.unwrap();
            let fetch_response = serialize(&resp).unwrap();

            return Ok(fetch_response);
        }
        ("fetch", "send", _) => {
            let state = resource_table.lock().await;
            let fetch_args: u32 = deserialize(payload).unwrap();
            let resp = op_fetch_send(state, fetch_args).await.unwrap();
            let fetch_response = serialize(&resp).unwrap();

            return Ok(fetch_response);
        }
        ("fetch", "read_body", _) => {
            let state = resource_table.lock().await;
            let fetch_args: FetchReadBody = deserialize(payload).unwrap();
            let fetch_response = op_fetch_read_body(state, fetch_args).await.unwrap();
            let fetch_response = serialize(&fetch_response).unwrap();

            return Ok(fetch_response);
        }
        ("fetch", "write_body", _) => {
            let state = resource_table.lock().await;
            let fetch_args: FetchWriteBody = deserialize(payload).unwrap();
            let resp = op_fetch_write_body(state, fetch_args).await.unwrap();
            let fetch_response = serialize(&resp).unwrap();

            return Ok(fetch_response);
        }
        _ => {}
    }

    Ok(vec![])
}
