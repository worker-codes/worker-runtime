use std::sync::Arc;
use futures::stream::StreamExt;
use object_store::{local::LocalFileSystem, ObjectStore, path::Path};

pub async fn get_wasm_file_function(path: String) -> anyhow::Result<Vec<u8>> {
    let path: Path = path.try_into().unwrap();
    // let temp_dir = TempDir::new("example").unwrap();
    let integration = LocalFileSystem::new_with_prefix("./wasm/").unwrap();
    let repo: Arc<dyn ObjectStore> = Arc::new(integration);

    let mut stream = repo.get(&path).await.unwrap().into_stream();

    let mut content = Vec::new();
    while let Some(bytes) = stream.next().await {
        content.extend_from_slice(&bytes.unwrap());
    }

    Ok(content)
}