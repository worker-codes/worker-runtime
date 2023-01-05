mod file_store;
use object_store::{ObjectStore, local::LocalFileSystem};
use wkr_common::{resources::{ResourceTable, Resource}, errors::Error};
use tokio::sync::Mutex;
use std::{sync::{Arc}, borrow::Cow};
use tempdir::TempDir;
use wapc_codec::messagepack::{deserialize, serialize};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
struct PutFile {
    path: String,
    content: Vec<u8>,
}

struct FileStoreResource(dyn ObjectStore);

impl Resource for FileStoreResource {
    fn name(&self) -> Cow<str> {
        "database".into()
    }
}

pub async fn process_database_ops(
    _id: u64,
    binding: &str,
    namespace: &str,
    operation: &str,
    payload: &[u8],
    resource_table: Arc<Mutex<ResourceTable>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {

    let temp_dir = TempDir::new("example").unwrap();
    let integration = LocalFileSystem::new_with_prefix(temp_dir.path()).unwrap();
    let repo: Arc<dyn ObjectStore> = Arc::new(integration);
   
    match (binding, namespace, operation) {
        ("storage", "file", "get") => {
            let path: String = deserialize(payload).unwrap();

            let response = file_store::get(repo, path).await.unwrap();
            Ok(response)
        },
        ("storage", "file", "put") => {
            let file: PutFile = deserialize(payload).unwrap();

            let _result = file_store::put(repo, file.path, file.content).await.unwrap();
            Ok(vec![])
        },
        ("storage", "file", "delete") => {
            let path: String = deserialize(payload).unwrap();

            let _result = file_store::delete(repo, path).await.unwrap();
            Ok(vec![])
        },
        ("storage", "file", "list") => {
            let path: String = deserialize(payload).unwrap();

            let result = file_store::list(repo, path).await.unwrap();
            let response = serialize(&result).unwrap();
            Ok(response)
        },

        _ => {
            Ok(vec![])
        }
    }
}