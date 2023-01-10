pub mod database;
use database::{connected_to_database, execute, query};
use quaint::single::Quaint;
use wkr_common::{resources::{ResourceTable, Resource}};
use std::{sync::{Arc}, borrow::Cow};
use serde::{Deserialize, Serialize};
use wapc_codec::messagepack::{deserialize, serialize};
use tokio::sync::Mutex;

#[derive(Deserialize, Serialize, Debug)]
struct ExecuteOptions{
    raw: bool,
}
#[derive(Deserialize, Serialize, Debug)]
struct ExecuteRequest {
    rid: u32,
    query: String,
    #[serde(with = "serde_bytes")]
    args: Option<Vec<u8>>,
    options: ExecuteOptions,
}

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    url: String,
    username: Option<String>,
    password: Option<String>,
    database: Option<String>,
    port: Option<u16>,
    host: Option<String>
}

#[derive(Deserialize, Serialize, Debug)]
struct ClientResponse {
    rid: u32,
}

struct DatabaseResource(Quaint);

impl Resource for DatabaseResource {
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
    match (binding, namespace, operation) {
        ("database", "connection", "open") => {
            let config: Config = deserialize(payload)?;

            // let url = format!("mysql://{}:{}@{}:{}/{}", config.username.unwrap(), config.password.unwrap(), config.host.unwrap(), config.port.unwrap(), config.database.unwrap());
            let conn = connected_to_database(config.url).await?;
            let mut table = resource_table.lock().await;
            let rid = table.add(DatabaseResource(conn));

            let client_response = ClientResponse { rid };
            let response = serialize(&client_response)?;
            Ok(response)
        }

        ("database", "connection", "close") => {

            let mut table = resource_table.lock().await;
            let mut payload = payload;
            let rid = rmp::decode::read_u32(&mut payload)?;
            table.close(rid)?;

            Ok(vec![])
        }

        ("database", "command", "query") => {
            let request: ExecuteRequest = deserialize(payload)?;
            let  rid = request.rid;

            let table = resource_table.lock().await;      
            let conn = table.get::<DatabaseResource>(rid)?;

            let conn = &conn.0;
            let query_sql = request.query;
            let args = request.args.unwrap_or(vec![]);
            let result = query(conn, &query_sql, args).await?;
            
            let response = serialize(&result)?;
            Ok(response)
        }

        ("database", "command", "execute") => {
            let request: ExecuteRequest = deserialize(payload)?;
            let  rid = request.rid;

            let table = resource_table.lock().await;    
            let conn = table.get::<DatabaseResource>(rid)?;
            let conn = &conn.0;
            let query_sql = request.query;
            let args = request.args.unwrap_or(vec![]);
            let result = execute(conn, &query_sql, args).await?;

            let response = serialize(&result)?;

            Ok(response)
        },
        _ => {
            Ok(vec![])
        }
    }
}
