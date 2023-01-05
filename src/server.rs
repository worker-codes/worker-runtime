mod error;
mod sse;
mod utils;

use axum::{
    body::{ Bytes, Full},
    extract::{Query, State, Path},
    headers::{authorization::{Bearer, Credentials}, HeaderName},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION, Method, Uri, HeaderValue},
    response::{sse::{ Sse}, Response },
    response::{Html, IntoResponse},
    routing::{any, get, post},
    Router,
};
use error::{ UserRepoError};
use serde::{Deserialize, Serialize};
use sse::{Broadcaster, ClientStream};
use utils::get_wasm_file_function;
use wkr_core::{create_function_engine_with_bytes};
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use moka::future::Cache;
use crate::error::AppError;
use wapc_codec::messagepack::{deserialize, serialize};
#[derive(Clone)]
struct AppState {
    #[allow(unused)]
    functions: HashMap<String, Vec<u8>>,
    cache: Cache<String, Vec<u8>> ,
    broadcaster: Arc<std::sync::Mutex<Broadcaster>>,
}

#[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct GuestRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct GuestResponse {
    status: u16,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();
    sse::print_jwt();

    let cache:Cache<String, Vec<u8>> = Cache::builder()
        // .max_size(10_000)
        // This cache will hold up to 100MiB of values.
        .max_capacity(100 * 1024 * 1024)
        // Time to live (TTL): 30 minutes
        .time_to_live(Duration::from_secs(30 * 60))
        // Time to idle (TTI):  5 minutes
        .time_to_idle(Duration::from_secs( 5 * 60))
        // Create the cache.
        .build();
    // Create a cache that can store up to 10,000 entries.
    // let cache:Cache<String, Vec<u8>> = Cache::new(10_000);
    let broadcaster = Broadcaster::create();
    let functions: HashMap<String, Vec<u8>> = HashMap::new();
    let shared_state = Arc::new(AppState {
        functions,
        broadcaster,
        cache
    });
    

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/sse", get(sse_handler))
        .route("/sse_publish", post(sse_publish_handler))
        // `POST /users` goes to `create_user`
        .route("/metric", any(metric_handler))
        .route("/add/:function", post(cache_function_handler))
        .route("/invoke/:function/:event", post(invoke_function_handler))
        .with_state(shared_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3333));
    tracing::debug!("listening on {}", addr);
    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    println!("Listening on http://{}", addr);
    if let Err(err) = server.await {
        eprintln!("Server error: {}", err);
    }
}

// basic handler that responds with a static string
async fn root() -> Html<&'static str> {    // "Hello, World!"

    Html(sse::HTML)
}
async fn cache_function_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let function = params.get("function").ok_or(UserRepoError::NotFound)?;
    // let event = params.get("event").ok_or(UserRepoError::NotFound)?;

    let cache =&state.cache;
    let wasm = get_wasm_file_function(function.to_string()).await.map_err(|_e| UserRepoError::InvalidFunction)?;

    let _function = cache.insert(function.to_string(), wasm).await;

    Ok(StatusCode::OK.into_response())
}

async fn invoke_function_handler(
    State(state): State<Arc<AppState>>,
    Path(params): Path<HashMap<String, String>>,
    method: Method,
    uri: Uri,
    headers_map: HeaderMap, 
    // Query(params): Query<HashMap<String, String>>,        
    body: Bytes
) -> Result<impl IntoResponse, AppError> {
    let function = params.get("function").ok_or(UserRepoError::NotFound)?;
    let event = params.get("event").ok_or(UserRepoError::NotFound)?;

    let cache =&state.cache;
    let function = cache.get(function).ok_or(UserRepoError::InvalidFunction)?;

    let method = method.to_string();
    let url = uri.to_string();

    let mut headers: Vec<(String, String)> = Vec::new();
    for (key, val) in headers_map.iter() {
        headers.push((key.to_string(), val.to_str().unwrap().to_string()));
    }

    let request_args = GuestRequest {
        method,
        url,
        headers,
        body: body.to_vec()
    };

    let resp = serialize(&request_args).map_err(|_e| UserRepoError::FailFunctionExecution)?;

    let mut environment = create_function_engine_with_bytes(function).await.map_err(|_e| UserRepoError::FailFunctionExecution)?;
    environment.init().await.map_err(|_e| UserRepoError::FailFunctionExecution)?;
    let guest_result = environment.call(&event, &resp).await.map_err(|_e| UserRepoError::FailFunctionExecution)?;

    let mut guest_response: GuestResponse = deserialize(&guest_result).map_err(|_e| UserRepoError::FailFunctionExecution)?;

    let mut response = Response::builder().status(guest_response.status);
    let headers = response.headers_mut().ok_or( UserRepoError::FailFunctionExecution)?;
    
    //loop over guest_response.headers
    while let Some((key, val)) = guest_response.headers.pop() {
        let key = HeaderName::from_bytes(key.as_bytes()).map_err(|_e| UserRepoError::FailFunctionExecution)?;
        let val = HeaderValue::from_str(&val).map_err(|_e| UserRepoError::FailFunctionExecution)?;
        headers.insert(key, val);
    };     
    
   let response = response
    .body(Full::from(guest_response.body))
    .map_err(|_e| UserRepoError::FailFunctionExecution)?;

    Ok(response.into_response())


}



async fn metric_handler(State(state): State<Arc<AppState>>) {
    // ...
}

async fn sse_publish_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
// ) -> Response<Body> {
) -> Response {
    
    let bearer = get_bearer_token(headers);
    let result = sse::publish(state.broadcaster.clone(), bearer, body).await;

    if let Ok(result) = result {
        // return (StatusCode::CREATED, Json(Value::String("result".to_owned())));
        result.into_response()
    } else {
        // return (StatusCode::UNAUTHORIZED, Json(Value::String("".to_owned())));
        StatusCode::UNAUTHORIZED.into_response()
    }   
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
// ) -> Sse<impl Stream<Item = Result<Event, anyhow::Error>>> {
) -> Response {

    let bearer = get_bearer_token(headers);
    
    let stream: anyhow::Result<ClientStream> = sse::sse(state.broadcaster.clone(), bearer, params).await;

    match stream {
        Ok(stream) => {
            Sse::new(stream).keep_alive(
                axum::response::sse::KeepAlive::new()
                    .interval(Duration::from_secs(1))
                    .text("keep-alive-text"),
            ).into_response()
        },
        Err(_err) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()   
        }
    }


}

fn get_bearer_token(headers: HeaderMap) -> Option<Bearer> {
    let bearer = if let Some(authorization) = headers.get(AUTHORIZATION) {
        Bearer::decode(&authorization)
    } else {
        None
    };
    bearer
}
