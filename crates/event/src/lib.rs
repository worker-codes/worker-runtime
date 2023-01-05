use anyhow::anyhow;
use wapc_codec::messagepack::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, collections::HashMap};
use tokio::sync::Mutex;
use wkr_common::resources::ResourceTable;

pub async fn process_event_ops(
    _id: u64,
    binding: &str,
    namespace: &str,
    operation: &str,
    payload: &[u8],
    _resource_table: Arc<Mutex<ResourceTable>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    match (binding, namespace, operation) {
        ("message", "broadcast_channel", "post") => {

            let data: String = deserialize(payload).unwrap();
           
            let mut params = HashMap::new();
            params.insert("topic", "https://example.com/books/1");
            params.insert("data", &data);

            let client = reqwest::Client::new();
            let res = client.post("https://localhost:3000/.well-known/mercure")
                .form(&params)
                .send().await?;

            let response = res.bytes().await?; 
            Ok(response.to_vec())
        } _ => {
            Ok(vec![])
        }
    }
}
