use futures::stream::StreamExt;
// use futures_util::stream::stream::StreamExt;
use anyhow::Result;
use object_store::{path::Path, ObjectStore};
use std::sync::Arc;

pub async fn get(repo: Arc<dyn ObjectStore>, path: String) -> Result<Vec<u8>> {
    let path: Path = path.try_into().unwrap();

    // fetch the bytes from object store
    let mut stream = repo.get(&path).await.unwrap().into_stream();

    let mut content = Vec::new();
    while let Some(bytes) = stream.next().await {
        content.extend_from_slice(&bytes.unwrap());
    }

    Ok(content)
}

pub async fn put(repo: Arc<dyn ObjectStore>, path: String, content: Vec<u8>) -> Result<()> {
    let path: Path = path.try_into().unwrap();

    // fetch the bytes from object store
    let result = repo.put(&path, content.into()).await.unwrap();

    Ok(result)
}

pub async fn list(repo: Arc<dyn ObjectStore>, path: String) -> Result<Vec<String>> {
    let prefix: Path = path.try_into().unwrap();

    let list_stream = repo.list(Some(&prefix)).await.expect("Error listing files");

    let list_stream = list_stream.map(|meta| {
        let result = meta.unwrap().location.to_string();
        // let result = result.as_bytes().to_vec();

        result
    });
    let content = list_stream.collect::<Vec<_>>().await;

    Ok(content)
}

pub async fn delete(repo: Arc<dyn ObjectStore>, path: String) -> Result<()> {
    let path: Path = path.try_into().unwrap();

    // fetch the bytes from object store
    let stream = repo.delete(&path).await.unwrap();

    Ok(stream)
}
