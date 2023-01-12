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

    let fut = request.send();

    let request_rid = resource_table
      .add(FetchRequestResource(Box::pin(fut)));

    // let cancel_handle_rid = resource_table.add(FetchCancelHandle(cancel_handle));

    // (request_rid, request_body_rid, Some(cancel_handle_rid))
    Ok(FetchReturn {
        request_rid,
        request_body_rid,
    })

    // // let body = res.bytes().await.unwrap();

    // let content_length = res.content_length();
    // let stream: BytesStream = Box::pin(
    //     res.bytes_stream()
    //         .map(|r| r.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))),
    // );
    // let stream_reader = StreamReader::new(stream);
    // let rid = resource_table.add(FetchResponseBodyResource2(stream_reader));

    // Ok(FetchResponse {
    //     status: status.as_u16(),
    //     status_text: status.canonical_reason().unwrap_or("").to_string(),
    //     headers: res_headers,
    //     url,
    //     request_body_rid: request_body_rid,
    //     // body:body.to_vec(),
    //     // body: ByteBuf::from(body),
    // })
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
    let rid = resource_table.add(FetchResponseBodyResource2(stream_reader));
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
struct FetchResponseBodyResource2(StreamReader<BytesStream, bytes::Bytes>);

impl Resource for FetchResponseBodyResource2 {
    fn name(&self) -> Cow<str> {
        "fetchResponseBody".into()
    }
}

// type CancelableResponseResult = Result<Result<Response, anyhow::Error>, wkr_common::async_cancel::Canceled>;

// struct FetchRequestResource(
//   Pin<Box<dyn Future<Output = CancelableResponseResult>>>,
// );

// impl Resource for FetchRequestResource {
//   fn name(&self) -> Cow<str> {
//     "fetchRequest".into()
//   }
// }

// struct FetchCancelHandle(Arc<CancelHandle>);

// impl Resource for FetchCancelHandle {
//   fn name(&self) -> Cow<str> {
//     "fetchCancelHandle".into()
//   }

//   fn close(self: Arc<Self>) {
//     self.0.cancel()
//   }
// }

// pub struct FetchRequestBodyResource {
//   body: AsyncRefCell<mpsc::Sender<std::io::Result<bytes::Bytes>>>,
//   cancel: CancelHandle,
// }

// impl Resource for FetchRequestBodyResource {
//   fn name(&self) -> Cow<str> {
//     "fetchRequestBody".into()
//   }

//   fn write(self: Arc<Self>, buf: BufView) -> AsyncResult<WriteOutcome> {
//     Box::pin(async move {
//       let bytes: bytes::Bytes = buf.into();
//       let nwritten = bytes.len();
//       let body = RcRef::map(&self, |r| &r.body).borrow_mut().await;
//       let cancel = RcRef::map(self, |r| &r.cancel);
//       body.send(Ok(bytes)).or_cancel(cancel).await?.map_err(|_| {
//         type_error("request body receiver not connected (request closed)")
//       })?;
//       Ok(WriteOutcome::Full { nwritten })
//     })
//   }

//   fn close(self: Arc<Self>) {
//     self.cancel.cancel()
//   }
// }

// type BytesStream =
//   Pin<Box<dyn Stream<Item = Result<bytes::Bytes, std::io::Error>> + Unpin+ Send+ Sync>>;

// struct FetchResponseBodyResource {
//   reader: AsyncRefCell<Peekable<BytesStream>>,
//   cancel: CancelHandle,
//   size: Option<u64>,
// }

// impl Resource for FetchResponseBodyResource {
//   fn name(&self) -> Cow<str> {
//     "fetchResponseBody".into()
//   }

//   fn read(self: Arc<Self>, limit: usize) -> AsyncResult<BufView> {
//     Box::pin(async move {
//       let reader = RcRef::map(&self, |r| &r.reader).borrow_mut().await;

//       let fut = async move {
//         let mut reader = Pin::new(reader);
//         loop {
//           match reader.as_mut().peek_mut().await {
//             Some(Ok(chunk)) if !chunk.is_empty() => {
//               let len = min(limit, chunk.len());
//               let chunk = chunk.split_to(len);
//               break Ok(chunk.into());
//             }
//             // This unwrap is safe because `peek_mut()` returned `Some`, and thus
//             // currently has a peeked value that can be synchronously returned
//             // from `next()`.
//             //
//             // The future returned from `next()` is always ready, so we can
//             // safely call `await` on it without creating a race condition.
//             Some(_) => match reader.as_mut().next().await.unwrap() {
//               Ok(chunk) => assert!(chunk.is_empty()),
//               Err(err) => break Err(type_error(err.to_string())),
//             },
//             None => break Ok(BufView::empty()),
//           }
//         }
//       };

//       let cancel_handle = RcRef::map(self, |r| &r.cancel);
//       fut.try_or_cancel(cancel_handle).await
//     })
//   }

//   fn size_hint(&self) -> (u64, Option<u64>) {
//     (self.size.unwrap_or(0), self.size)
//   }

//   fn close(self: Arc<Self>) {
//     self.cancel.cancel()
//   }
// }

// impl Resource for FetchRequestBodyResource2 {
//     fn name(&self) -> Cow<str> {
//         "database".into()
//     }
// }

// pub struct FetchRequestBodyResource {
//     body: AsyncRefCell<Arc<mpsc::Sender<std::io::Result<Vec<u8>>>>>,
//     cancel: CancelHandle,
// }

// impl Resource for FetchRequestBodyResource {
//     fn name(&self) -> Cow<str> {
//         "fetchRequestBody".into()
//     }

//     fn write(self: Arc<Self>, buf: Vec<u8>) -> AsyncResult<usize> {
//         Box::pin(async move {
//             let data = buf.to_vec();
//             let len = data.len();
//             let body = RcRef::map(&self, |r| &r.body).borrow_mut().await;
//             let cancel = RcRef::map(self, |r| &r.cancel);
//             body.send(Ok(data))
//                 .or_cancel(cancel)
//                 .await?
//                 .map_err(|_| type_error("request body receiver not connected (request closed)"))?;

//             Ok(len)
//         })
//     }

//     fn close(self: Arc<Self>) {
//         self.cancel.cancel()
//     }
// }

// type BytesStream = Pin<Box<dyn Stream<Item = Result<bytes::Bytes, std::io::Error>> + Unpin>>;

// struct FetchResponseBodyResource {
//     reader: AsyncRefCell<StreamReader<BytesStream, bytes::Bytes>>,
//     cancel: CancelHandle,
// }

// impl Resource for FetchResponseBodyResource {
//     fn name(&self) -> Cow<str> {
//         "fetchResponseBody".into()
//     }

//     fn read(self: Arc<Self>, mut buf: Vec<u8>) -> AsyncResult<usize> {
//         Box::pin(async move {
//             let mut reader = RcRef::map(&self, |r| &r.reader).borrow_mut().await;
//             let cancel = RcRef::map(self, |r| &r.cancel);
//             let read = reader.read(&mut buf).try_or_cancel(cancel).await?;
//             Ok(read)
//         })
//     }

//     fn close(self: Arc<Self>) {
//         self.cancel.cancel()
//     }
// }

struct HttpClientResource {
    client: Client,
}

// pub struct FetchRequestBodyResource {
//     body: AsyncRefCell<mpsc::Sender<std::io::Result<bytes::Bytes>>>,
//     cancel: CancelHandle,
//   }

//   impl Resource for FetchRequestBodyResource {
//     fn name(&self) -> Cow<str> {
//       "fetchRequestBody".into()
//     }

//     fn write(self: Rc<Self>, buf: BufView) -> AsyncResult<WriteOutcome> {
//       Box::pin(async move {
//         let bytes: bytes::Bytes = buf.into();
//         let nwritten = bytes.len();
//         let body = RcRef::map(&self, |r| &r.body).borrow_mut().await;
//         let cancel = RcRef::map(self, |r| &r.cancel);
//         body.send(Ok(bytes)).or_cancel(cancel).await?.map_err(|_| {
//           type_error("request body receiver not connected (request closed)")
//         })?;
//         Ok(WriteOutcome::Full { nwritten })
//       })
//     }

//     fn close(self: Rc<Self>) {
//       self.cancel.cancel()
//     }
//   }

//   type BytesStream =
//     Pin<Box<dyn Stream<Item = Result<bytes::Bytes, std::io::Error>> + Unpin>>;

//   struct FetchResponseBodyResource {
//     reader: AsyncRefCell<Peekable<BytesStream>>,
//     cancel: CancelHandle,
//     size: Option<u64>,
//   }

//   impl Resource for FetchResponseBodyResource {
//     fn name(&self) -> Cow<str> {
//       "fetchResponseBody".into()
//     }

//     fn read(self: Rc<Self>, limit: usize) -> AsyncResult<BufView> {
//       Box::pin(async move {
//         let reader = RcRef::map(&self, |r| &r.reader).borrow_mut().await;

//         let fut = async move {
//           let mut reader = Pin::new(reader);
//           loop {
//             match reader.as_mut().peek_mut().await {
//               Some(Ok(chunk)) if !chunk.is_empty() => {
//                 let len = min(limit, chunk.len());
//                 let chunk = chunk.split_to(len);
//                 break Ok(chunk.into());
//               }
//               // This unwrap is safe because `peek_mut()` returned `Some`, and thus
//               // currently has a peeked value that can be synchronously returned
//               // from `next()`.
//               //
//               // The future returned from `next()` is always ready, so we can
//               // safely call `await` on it without creating a race condition.
//               Some(_) => match reader.as_mut().next().await.unwrap() {
//                 Ok(chunk) => assert!(chunk.is_empty()),
//                 Err(err) => break Err(type_error(err.to_string())),
//               },
//               None => break Ok(BufView::empty()),
//             }
//           }
//         };

//         let cancel_handle = RcRef::map(self, |r| &r.cancel);
//         fut.try_or_cancel(cancel_handle).await
//       })
//     }

//     fn size_hint(&self) -> (u64, Option<u64>) {
//       (self.size.unwrap_or(0), self.size)
//     }

//     fn close(self: Rc<Self>) {
//       self.cancel.cancel()
//     }
//   }
