use anyhow::Error;
use http::header::ACCEPT_ENCODING;
use http::header::HeaderName;
use http::header::CONTENT_LENGTH;
use http::header::HOST;
use http::HeaderMap;
use http::HeaderValue;
use http::header::RANGE;
use reqwest::Body;
// use reqwest::Client;
use reqwest::Client;
use reqwest::Method;
use reqwest::Request;
use reqwest::RequestBuilder;
use reqwest::Response;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::MutexGuard;
// use wkr_common::async_cancel::CancelFuture;
// use wkr_common::async_cancel::CancelHandle;
use wkr_common::async_cancel::CancelTryFuture;
use wkr_common::async_cancel::Canceled;
use wkr_common::async_cell::AsyncRefCell;
use wkr_common::async_cell::RcRef;
use wkr_common::resources::ResourceId;
// use wkr_common::io::BufView;
// use wkr_common::io::WriteOutcome;
use wkr_common::resources::type_error;
use wkr_common::resources::Resource;
use wkr_common::resources::ResourceTable;
// use wapc::ResourceTable;
use std::borrow::Cow;
use std::cmp::min;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;
// use bytes::Bytes;
use serde_bytes::ByteBuf;
use serde_bytes::Bytes;
// use wapc::Resource;
use futures::Future;
use std::pin::Pin;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
// use tokio_stream::StreamExt;
use futures::StreamExt;
use tokio_util::io::StreamReader;
use wapc_codec::messagepack::{deserialize, serialize};

/// Returned by resource read/write/shutdown methods
pub type AsyncResult<T> = Pin<Box<dyn Future<Output = Result<T, Error>>>>;

// struct FetchRequestResource(String);

// impl Resource for FetchRequestResource {
//     fn name(&self) -> Cow<str> {
//         "fetchRequest".into()
//     }
// }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchReturn {
    request_rid: ResourceId,
    request_body_rid: Option<ResourceId>,
    // cancel_handle_rid: Option<ResourceId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    client_rid: Option<u32>,
    has_body: bool,
    body_length: Option<u64>,
    data: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HostRequest {
    method: String,
    redirect: String,
    url: String,
    headers: Option<Vec<(String, String)>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchArgs {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    client_rid: Option<u32>,
    has_body: bool,
    body_length: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
// #[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    // status: u16,
    // status_text: String,
    // headers: Vec<(String, String)>,
    // url: String,
    // request_body_rid: Option<u32>,
    // body: Vec<u8>,
    // body: serde_bytes::ByteBuf,
    //   body:Option<Bytes>,
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    url: String,
    response_rid: ResourceId,
    content_length: Option<u64>,
}

struct FetchClientResource(Client);

impl Resource for FetchClientResource {
    fn name(&self) -> Cow<str> {
        "fetchClient".into()
    }
}

struct FetchRequestBuilderResource(Arc<Mutex<RequestBuilder>>);

impl Resource for FetchRequestBuilderResource {
    fn name(&self) -> Cow<str> {
        "fetchRequestBuilder".into()
    }
}
struct FetchResponseResource(Response);

impl Resource for FetchResponseResource {
    fn name(&self) -> Cow<str> {
        "fetchResponse".into()
    }
}

type CancelableResponseResult = Result<Response, reqwest::Error>;
// type BytesStream =
//     Pin<Box<dyn Stream<Item = Result<bytes::Bytes, std::io::Error>> + Unpin + Send + Sync>>;
struct FetchRequestResource(
  Pin<Box<dyn Future<Output = CancelableResponseResult> + Unpin + Send + Sync >>,
);

impl Resource for FetchRequestResource {
  fn name(&self) -> Cow<str> {
    "fetchRequest".into()
  }
}

struct StringResource(String);
  
  impl Resource for StringResource {
    fn name(&self) -> Cow<str> {
      "fetchRequest".into()
    }
  }

// struct FetchCancelHandle(Arc<CancelHandle>);

// impl Resource for FetchCancelHandle {
//   fn name(&self) -> Cow<str> {
//     "fetchCancelHandle".into()
//   }

//   fn close(self: Arc<Self>) {
//     self.0.cancel()
//   }
// }
pub async fn op_fetch(
    mut resource_table: MutexGuard<'_, ResourceTable>,
    args: FetchRequest,
) -> Result<FetchReturn, Error> {
    let method = Method::from_bytes(&args.method.as_bytes()).unwrap();
    let url = Url::parse(&args.url).unwrap();

    let client = Client::new();
    let mut request = client.request(method.clone(), url);

    let request_body_rid = if args.has_body {
        match args.data {
            None => {
                // If no body is passed, we return a writer for streaming the body.
                let (tx, rx) = mpsc::channel::<std::io::Result<bytes::Bytes>>(1);

                // If the size of the body is known, we include a content-length
                // header explicitly.
                if let Some(body_size) = args.body_length {
                    request = request.header(CONTENT_LENGTH, HeaderValue::from(body_size))
                }

                request = request.body(Body::wrap_stream(ReceiverStream::new(rx)));

                let request_body_rid = resource_table.add(FetchRequestBodyResource2(tx));
                println!("request_body_rid: {}", request_body_rid);
                // let request_body_rid =
                //   resource_table.add(FetchRequestBodyResource {
                //     body: AsyncRefCell::new(tx),
                //     cancel: CancelHandle::default(),
                //   });

                Some(request_body_rid)
            }
            Some(data) => {
                // If a body is passed, we use it, and don't return a body for streaming.
                request = request.body(Vec::from(&*data));
                None
            }
        }
    } else {
        // POST and PUT requests should always have a 0 length content-length,
        // if there is no body. https://fetch.spec.whatwg.org/#http-network-or-cache-fetch
        if matches!(method, Method::POST | Method::PUT) {
            request = request.header(CONTENT_LENGTH, HeaderValue::from(0));
        }
        None
    };

    let mut header_map = HeaderMap::new();
    for (key, value) in args.headers {
        let name = HeaderName::from_bytes(&key.as_bytes())
            .map_err(|err| anyhow::anyhow!(err.to_string()))
            .unwrap();
        let v = HeaderValue::from_bytes(&value.as_bytes())
            .map_err(|err| anyhow::anyhow!(err.to_string()))
            .unwrap();
        if !matches!(name, HOST | CONTENT_LENGTH) {
            header_map.append(name, v);
        }
    }

    if header_map.contains_key(RANGE) {
        // https://fetch.spec.whatwg.org/#http-network-or-cache-fetch step 18
        // If httpRequest’s header list contains `Range`, then append (`Accept-Encoding`, `identity`)
        header_map.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    }
    request = request.headers(header_map);

    // let res = request.send().await.unwrap();

    // let status = res.status();
    // let url = res.url().to_string();
    // let mut res_headers = Vec::new();
    // for (key, val) in res.headers().iter() {
    //     // let key_bytes: &[u8] = key.as_ref();
    //     res_headers.push((key.to_string(), val.to_str().unwrap().to_string()));
    // }

    // let cancel_handle = CancelHandle::new_rc();
    // let cancel_handle_ = cancel_handle.clone();

    // let fut = async move {
    //   request
    //     .send()
    //     // .or_cancel(cancel_handle_)
    //     .await
    //     .map_err(|err| type_error(err.to_string()));
    // };
    // let request_rid = resource_table
    // .add(StringResource(String::from("hello")));

    let fut = request.send();

    let request_rid = resource_table
      .add(FetchRequestResource(Box::pin(fut)));

      println!("request_rid: {}", request_rid);

    // let cancel_handle_rid = resource_table.add(FetchCancelHandle(cancel_handle));

    // (request_rid, request_body_rid, Some(cancel_handle_rid))
    Ok(FetchReturn {
        request_rid,
        request_body_rid,
    })

}

pub async fn op_fetch_send(
    mut resource_table: MutexGuard<'_, ResourceTable>,
    rid: ResourceId,
  ) -> anyhow::Result<FetchResponse> {
    let request = resource_table
      .take::<FetchRequestResource>(rid)?;
  
    let request = Arc::try_unwrap(request)
      .ok()
      .expect("multiple op_fetch_send ongoing");
  
    let res = match request.0.await {
      Ok(res) => res,
      Err(err) => return Err(type_error(err.to_string())),
    //   Err(_) => return Err(type_error("request was cancelled")),
    };
  
    //debug!("Fetch response {}", url);
    let status = res.status();
    let url = res.url().to_string();
    let mut res_headers:Vec<(String, String)> = Vec::new();
    for (key, val) in res.headers().iter() {
      res_headers.push((key.as_str().into(), val.to_str().unwrap().to_string()));
    }
  
    let content_length = res.content_length();
  
    let stream: BytesStream = Box::pin(res.bytes_stream().map(|r| {
      r.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }));
    let stream_reader = StreamReader::new(stream);
    //create a tokio thread to read the stream
    let (tx, rx) = mpsc::channel(1);
    tokio::spawn(async move {
        let mut stream_reader = stream_reader;
        let mut tx = tx.clone();
        loop {
            // rx.recv().await;
            let mut buf = [0; 1024];
            let n = stream_reader.read(&mut buf).await.unwrap();
            
            let bytes = bytes::Bytes::copy_from_slice(&buf[..n]);
            tx.send(Ok(bytes)).await.unwrap();
            if n == 0 {
                break;
            }
        }
        // let mut buf = [0; 1024];
        // read.read_exact(&mut buf).await?;


    });
    let rid = resource_table.add(FetchResponseBodyResource2(rx));
    //

    // let rid = resource_table.add(FetchResponseBodyResource2(stream_reader));
    // let rid = resource_table
    //   .add(FetchResponseBodyResource {
    //     reader: AsyncRefCell::new(stream.peekable()),
    //     cancel: CancelHandle::default(),
    //     size: content_length,
    //   });
  
    Ok(FetchResponse {
      status: status.as_u16(),
      status_text: status.canonical_reason().unwrap_or("").to_string(),
      headers: res_headers,
      url,
      response_rid: rid,
      content_length,
    })
  }

struct FetchRequestBodyResource2(mpsc::Sender<std::io::Result<bytes::Bytes>>);
impl Resource for FetchRequestBodyResource2 {
    fn name(&self) -> Cow<str> {
        "database".into()
    }
}

type BytesStream =
    Pin<Box<dyn Stream<Item = Result<bytes::Bytes, std::io::Error>> + Unpin + Send + Sync>>;
// type BytesStream =
//   Pin<Box<dyn Stream<Item = Result<bytes::Bytes, std::io::Error>> + Unpin>>;
// struct FetchResponseBodyResource2(BytesStream);
struct FetchResponseBodyResource2(tokio::sync::mpsc::Receiver<Result<bytes::Bytes, std::io::Error>>);
// struct FetchResponseBodyResource2(StreamReader<BytesStream, bytes::Bytes>);

impl Resource for FetchResponseBodyResource2 {
    fn name(&self) -> Cow<str> {
        "fetchResponseBody".into()
    }
}

pub struct HttpClientResource {
    client: Client,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchReadBody{
    rid: ResourceId,
    size: u64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchReadBodyReturn{
    #[serde(with = "serde_bytes")]
    chunk: Vec<u8>,
    size: u64,
    rid: ResourceId,
}



pub async fn op_fetch_read_body(
    mut resource_table: MutexGuard<'_, ResourceTable>,
    args: FetchReadBody,
  ) -> anyhow::Result<FetchReadBodyReturn> {
    println!("op_fetch_read_body3333333: {}", args.rid);
    let response_body = resource_table
      .take::<FetchResponseBodyResource2>(args.rid)?;
  
    let mut response_body = Arc::try_unwrap(response_body)
        .ok()
        .expect("multiple op_fetch_read_body ongoing");
    let mut response_body = response_body.0;

    let bytes = response_body.recv().await.unwrap().unwrap();
    println!("bytes!!!!!!!!!!: {}", bytes.len());

    // // let buffer = &mut vec![];
    // let response_body = &mut response_body.0;
    // let mut chunk =    vec![0; args.size as usize];
    // // let body = response_body.0.read_to_end(buffer).await?;
    // let size = response_body.read(&mut chunk).await?;

    let rid = resource_table.add(FetchResponseBodyResource2(response_body));
    // let rid = resource_table.add(response_body);
    println!("op_fetch_read_body!!!!!!!!!!: {}", rid);
  
    Ok(FetchReadBodyReturn{
        chunk: bytes.to_vec(),
        size: bytes.len() as u64,
        rid:rid
    })
  }




  #[derive(Serialize, Deserialize, Debug)]
pub struct FetchWriteBody{
    rid: ResourceId,
    #[serde(with = "serde_bytes")]
    chunk: Vec<u8>,
    size: u64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchWriteBodyReturn{
    size: u64,
    rid: ResourceId,
}
  pub async fn op_fetch_write_body(
    mut resource_table: MutexGuard<'_, ResourceTable>,
    args: FetchWriteBody,
  ) -> anyhow::Result<FetchWriteBodyReturn> {
    println!("op_fetch_read_body3333333: {}", args.rid);
    let resquest_body = resource_table
      .take::<FetchRequestBodyResource2>(args.rid)?;
  
    let resquest_body = Arc::try_unwrap(resquest_body)
        .ok()
        .expect("multiple op_fetch_read_body ongoing");
    let resquest_body = resquest_body.0;

    let chunk = bytes::Bytes::copy_from_slice(&args.chunk);
    let _bytes = resquest_body.send(Ok(chunk)).await.unwrap();
    // println!("bytes!!!!!!!!!!: {}", bytes.len());


    let rid = resource_table.add(FetchRequestBodyResource2(resquest_body));
    // let rid = resource_table.add(resquest_body);
    println!("op_fetch_read_body!!!!!!!!!!: {}", rid);
  
    Ok(FetchWriteBodyReturn{
        size: args.size as u64,
        rid:rid
    })
  }
