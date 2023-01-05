pub mod auth;
// pub mod error;
use anyhow::Result;
use auth::{PublisherClaims, PublisherMercure, SubscriberClaims, SubscriberMercure};
use axum::body::{Body, Bytes};
use axum::headers::authorization::Bearer;
use axum::http::{ Response};
use axum::response::sse::Event;
use mpsc::Sender;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc::{self, Receiver};
use tokio::time::{interval_at, Instant};
use urlpattern::{UrlPattern, UrlPatternInit, UrlPatternMatchInput};
use uuid::Uuid;

use futures::Stream;
use std::sync::Mutex;

use crate::error::Error;

pub const HTML: &str = r#"
<!DOCTYPE html>
<html>
    <head>
        <title>Hyper-usse Demo</title>
        <meta charset="utf-8" />
        <script type="application/javascript" src="https://cdn.jsdelivr.net/npm/event-source-polyfill@1.0.8/src/eventsource.min.js"></script>
    </head>
    <body>
        <h1>Hyper-usse Demo</h1>
        <p>Incoming events:</p>
        <ul id="list"></ul>
        <script>
            const subscriberJws = 'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJzdWIiOiJ1aWQiLCJtZXJjdXJlIjp7InN1YnNjcmliZSI6WyJodHRwczovL2V4YW1wbGUuY29tL215LXByaXZhdGUtdG9waWMiXSwicGF5bG9hZCI6bnVsbH0sImV4cCI6OTk5OTk5OTk5OTk5OTk5fQ.DgMW8hnz4eWCj7Q_Zuy_Ibn2JI5UjBhpbF5N7QzW0-aHbKv5RbM-XUk8jyzGhq9rZhZqrD0xno1DWvYeFWEreA';
            const endpoint = 'https://example.com'; // PUT YOUR SERVER URL
            // const url = new URL('http://localhost:8011/.well-known/mercure');
            const url = new URL('http://localhost:3333/sse');

            // Add topics to listen to
            url.searchParams.append('topic', `${endpoint}` + '/users/1234');
            // url.searchParams.append('topic', `${endpoint}` + '/books/{id}');
            url.searchParams.append('topic', `${endpoint}` + '/books/:id');
            url.searchParams.append('topic', `https://example.com/my-private-topic`);

            let server = new EventSource(url);
            // var server = new EventSourcePolyfill(
            //     url,
            //     {
            //       headers: {
            //         Authorization: `Bearer ${subscriberJws}`
            //       }
            //     }
            //   );
            server.onmessage = event => {
                let elem = document.createElement("li");
                elem.innerHTML = event.data;
                document.getElementById("list").appendChild(elem);
            };
            server.addEventListener('ping', event =>{
                let elem = document.createElement("li");
                elem.innerHTML = "PING:"+event.data;
                document.getElementById("list").appendChild(elem);
            }, false);
        </script>
    </body>
</html>"#;

// https://demo.mercure.rocks/.well-known/mercure?topic=https%3A%2F%2Fdemo.mercure.rocks%2Fdemo%2Fnovels%2F%7Bid%7D.jsonld&topic=https%3A%2F%2Fdemo.mercure.rocks%2Fdemo%2Fbooks%2F1.jsonld&topic=foo

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Publication {
    data: Option<String>,
    private: bool,
    id: Option<String>,
    r#type: Option<String>,
    retry: Option<u64>,
    topic: Vec<String>,
}

fn every<T, I>(v: I) -> bool
where
    I: IntoIterator<Item = T>,
    T: std::ops::Not<Output = bool>,
{
    v.into_iter().all(|x| !!x)
}

pub async fn publish(    
    broadcaster: Arc<Mutex<Broadcaster>>,
    bearer: Option<Bearer>,
    body: Bytes,
) -> Result<Response<Body>> {    
    // Get the application/x-www-form-urlencoded body and convert it to Publication type
    let mut publication = to_publication(body);

    let message_id;
    if let Some(id) = publication.id.clone() {
        message_id = id;
    } else {
        message_id = Uuid::new_v4().to_string();
        publication.id = Some(message_id.clone());
    };

    // Publishers MUST be authorized to dispatch updates to the hub, and MUST prove that they are authorized to send updates for the specified topics.
    // To be allowed to publish an update, the JWS presented by the publisher MUST contain a claim called mercure, and this claim MUST contain a publish key.
    let authorized_topics = auth::authorize_publisher(bearer).await;

    // Check if publisher has 'mercure.publish' in token if it is not defined, then the publisher MUST NOT be authorized to dispatch any update
    if let Ok(authorized_topics) = authorized_topics {
        let authorized = authorized_topics.publish.clone();
        let candidates = publication.topic.clone();

        // if 'mercure.publish' contains an empty array, the publisher MUST NOT be authorized to publish private updates, but can publish public updates for all topics.
        if publication.private {
            if check_if_all_topic_is_authorized(candidates, authorized)? {
                let lock = broadcaster.lock().map_err(|_| Error::BroadcasterLockError)?;
                lock.send(publication)?;

                return Ok(Response::builder().body(Body::from(message_id))?)
            } else {
                return Ok(Response::builder().status(403).body(Body::empty())?)
            }
        } else {
            broadcaster.lock().map_err(|_| Error::BroadcasterLockError)?.send(publication)?;

            return Ok(Response::builder().body(Body::from(message_id))?)
        }
    } else {
        return Ok(Response::builder().status(403).body(Body::empty())?)
    }
}

// The hub MUST check that every topics of the update to dispatch matches at least one of the topic selectors contained in mercure.publish.
/// If the publisher is not authorized for all the topics of an update, the hub MUST NOT dispatch
/// the update (even if some topics in the list are allowed) and MUST return a 403 HTTP status code.
/* fn check_if_all_topic_is_authorized(candidates: Vec<String>, authorized: Vec<String>) -> Result<bool> {
    let mut matches: Vec<bool> = vec![];

    candidates.iter().for_each(|candidate| {
        let found = authorized
            .clone()
            .into_iter()
            .find(|topic| uri_matcher(topic.to_string(), candidate.to_string())?);

        if let Some(_f) = found {
            matches.push(true)
        } else {
            matches.push(false)
        }
    });
    let matched = every(matches.into_iter());

    Ok(matched)
} */
fn check_if_all_topic_is_authorized(candidates: Vec<String>, authorized: Vec<String>) -> Result<bool> {
    let mut matches: Vec<bool> = vec![];

    let mut candidates = candidates.into_iter();
    let mut authorized = authorized.into_iter();
    while let Some(candidate) = candidates.next() {
        while let Some(topic) = authorized.next() {
            let found = uri_matcher(topic.to_string(), candidate.to_string())?;
            matches.push(found)
        }
    }

    let matched = every(matches.into_iter());

    Ok(matched)
}

fn to_publication(body: Bytes) -> Publication {
    // Convert body to vector of  tuples
    let params = form_urlencoded::parse(body.as_ref())
        .into_owned()
        .collect::<Vec<(String, String)>>();

    // Convert the tuple to Publication struct
    let publication = {
        let mut data: Option<String> = None;
        let mut private: bool = false;
        let mut id: Option<String> = None;
        let mut r#type: Option<String> = None;
        let mut retry: Option<u64> = None;
        let mut topic: Vec<String> = Vec::new();

        // loop over vector and match the required fields
        params.into_iter().for_each(|(key, value)| {
            if key == "private" {
                private = true;
            } else if key == "data" {
                data = Some(value);
            } else if key == "id" {
                id = Some(value);
            } else if key == "type" {
                r#type = Some(value);
            } else if key == "retry" {
                retry = Some(value.parse::<u64>().unwrap());
            } else if key == "topic" {
                topic.push(value);
            }
        });

        Publication {
            data,
            private,
            id,
            r#type,
            retry,
            topic,
        }
    };
    publication
}

/// To receive updates marked as private, a subscriber MUST prove that it is authorized for at least one of the topics of this update.
/// If the subscriber is not authorized to receive an update marked as private, it MUST NOT receive it.
pub async fn sse(
    broadcaster: Arc<Mutex<Broadcaster>>,
    bearer: Option<Bearer>,
    params: HashMap<String, String>,
) -> Result<ClientStream> {
    // let (parts, _body) = request.into_parts();

    //To receive updates marked as private, the JWS presented by the subscriber MUST have a claim named mercure with a key named subscribe that contains an array of topic selectors.
    let authorized_topics = auth::authorize_subscriber(bearer).await;

    let authorized_topics = match authorized_topics {
        Ok(val) => Some(val),
        Err(_e) => None,
    };

    // From the subscribers query params and find the topics that the subscriber wants messages from
    let topics = find_topic_from_params(params);

    // Create Subscriber and return body stream
    let mut broadcaster = broadcaster
        .lock()
        .map_err(|_| Error::BroadcasterLockError)?;        

     let stream= broadcaster.new_client(topics, authorized_topics)?;

    // Return stream to subscriber
    Ok(stream)
}

fn find_topic_from_params(params: HashMap<String, String>) -> Vec<String> {
    let mut topics: Vec<String> = vec![];
    params.iter().for_each(|(key, value)| {
        if key == "topic" {
            topics.push(value.to_string())
        }
    });
    topics
}

#[derive(Clone)]
pub struct Subscriber {
    // topics: Vec<String>,
    private_topics: Option<SubscriberMercure>,
    topics: Vec<String>,
    connection: Sender<Event>,
}

#[derive(Clone)]
pub struct Broadcaster {
    pub subscribers: Vec<Subscriber>,
}

impl Broadcaster {
    pub fn create() -> Arc<Mutex<Self>> {
        // Data ≃ Arc
        let me = Arc::new(Mutex::new(Broadcaster::new()));

        // ping subscribers every 10 seconds to see if they are alive
        Broadcaster::spawn_ping(me.clone());

        me
    }

    fn new() -> Self {
        Broadcaster {
            subscribers: Vec::new(),
        }
    }

    fn spawn_ping(me: Arc<Mutex<Self>>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut task = interval_at(Instant::now(), Duration::from_secs(10));
            loop {
                let _now = task.tick().await;
                me.lock().unwrap().remove_stale_subscribers();
            }
        })
    }

    fn remove_stale_subscribers(&mut self) {
        let mut ok_subscribers = Vec::new();

        for client in self.subscribers.iter() {
            //Bytes::from("data:\nevent:PING\n\n")
            let event = Event::default().data("PING");
            let result = client.connection.clone().try_send(event);

            if let Ok(()) = result {
                ok_subscribers.push(client.clone());
            }
        }
        self.subscribers = ok_subscribers;
    }

    fn new_client(
        &mut self,
        topics: Vec<String>,
        private_topics: Option<SubscriberMercure>,
    ) -> Result<ClientStream> {
        //Create channel to send messages to response body
        let (tx, rx) = mpsc::channel(100);

        let event = Event::default().data("connected\n\n");
        //Send connnection message
        tx.clone().try_send(event).map_err(|_| Error::EventSendMessage)?;

        //Create subscriber add to list
        let subscriber = Subscriber {
            private_topics: private_topics.clone(),
            topics: topics.clone(),
            connection: tx,
        };
        self.subscribers.push(subscriber);

        //return a stream receiver
        Ok(ClientStream(rx))
    }

    fn send(&self, publication: Publication)-> Result<()> {
        let candidates = publication.topic.clone();
        let msg = encode_msg(publication.clone());

        // Dispatch message to subscribers
        // self.subscribers.iter().for_each(|subscriber| {
        for subscriber in &self.subscribers {
            let msg = msg.clone();
            let subscriber_topics = subscriber.topics.clone();
            let subscriber_private_topics = subscriber.private_topics.clone();

            // Check if subscriber has subscription that  matches the publisher topics
            let match_topic = find_subscriber_topic(subscriber_topics, candidates.clone())?;

            // Send request to subscriber if topic is found
            if let Some(_t) = match_topic {
                //contains an empty array, the publisher MUST NOT be authorized to publish private updates, but can publish public updates for all topics.
                if publication.private {
                    // If publication is marked a private, check that the subsciber is authorized for that topic
                    // esle the publication is public and send message to subsciber.

                    if let Some(pt) = subscriber_private_topics {
                        // Check if the subsciber has private subscription the topic
                        let found = find_subscriber_private_topic(pt.subscribe, candidates.clone())?;

                        if let Some(_topic) = found {
                            // Send private message to subsriber
                            subscriber.connection.try_send(msg.clone()).unwrap_or(());
                        }
                    }
                } else {
                    subscriber.connection.try_send(msg.clone()).unwrap_or(());
                }
            }
        }
        Ok(())
    }
}

fn find_subscriber_private_topic(topics: Vec<String>, candidate: Vec<String>) -> Result<Option<String>> {
    for topic in topics {
        if find_bool(topic.clone(), candidate.clone())? {
           return Ok(Some(topic));
        }
    }
    Ok(None)
    // let found = pt
    //     .subscribe
    //     .into_iter()
    //     .find(|topic| find_bool(topic.to_string(), candidate.clone()));
    // found
}

fn find_subscriber_topic(topics: Vec<String>, candidate: Vec<String>) -> Result<Option<String>> {

    for topic in topics {
        if find_bool(topic.clone(), candidate.clone())? {
           return Ok(Some(topic));
        }
    }

    Ok(None)
    // let found = topics
    //     .into_iter()
    //     .find(|topic| find_bool(topic.to_string(), candidate.clone()));
    // found
}

pub struct ClientStream(Receiver<Event>);

impl Stream for ClientStream {
    type Item = Result<Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_recv(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
fn encode_msg(publication: Publication) -> Event {
    let mut event = Event::default();

    if let Some(val) = publication.data {
        event = event.data(val);
    } else {
        event = event.data("");
    }

    if let Some(val) = publication.r#type {
        event = event.event(val);
    }

    if let Some(val) = publication.id {
        event = event.id(val);
    } else {
        // event = event.data("");
        event = event.id("");
    }

    if let Some(val) = publication.retry {
        let time = Duration::from_secs(val.into());
        event = event.retry(time);
    }

    event
}

// candidates(topics) -- authorized
fn uri_matcher(candidates: String, authorized: String) -> Result<bool> {
    let candidates = candidates.parse()?;
    // Create the UrlPattern to match against.
    let init = UrlPatternInit {
        base_url: Some(candidates),
        ..Default::default()
    };
    let pattern = <UrlPattern>::parse(init)?;

    // Match the pattern against a URL.
    let authorized = authorized.parse()?;
    let result = pattern.test(UrlPatternMatchInput::Url(authorized))?;

    Ok(result)

    /*     let result = matcher(&pattern, &url);
    if let Some(_e) = result {
        return true;
    } else {
        return false;
    } */
}

fn find_bool(topic: String, candidates: Vec<String>) -> Result<bool> {
    // let found = candidates
    //     .into_iter()
    //     .find(|c| uri_matcher(topic.to_string(), c.to_string()));
    // if let Some(_t) = found {
    //     true
    // } else {
    //     false
    // }

    let mut matches: Vec<bool> = vec![];

    let mut candidates = candidates.into_iter();
    while let Some(candidate) = candidates.next() {
        let found = uri_matcher(topic.to_string(), candidate.to_string())?;
        matches.push(found)
    }

    let matched = every(matches.into_iter());

    Ok(matched)
}

pub fn print_jwt() {
    let publish_claims = PublisherClaims {
        sub: "uid".to_owned(),
        mercure: PublisherMercure {
            publish: vec![
                "https://example.com/my-private-topic".to_string(),
                // "https://example.com/my-private-topic/:user".to_string(),
            ],
        },
        exp: 999999999999999,
    };

    let subscriber_claims = SubscriberClaims {
        sub: "uid".to_owned(),
        mercure: SubscriberMercure {
            subscribe: vec![
                "https://example.com/my-private-topic".to_string(),
                // "https://example.com/my-private-topic/:user".to_string(),
            ],
            payload: None,
        },
        exp: 999999999999999,
    };
    println!("publish {:#?}", auth::create_jwt(&publish_claims));

    println!("subcribe {:#?}", auth::create_jwt(&subscriber_claims));
}

// #[derive(Debug, Default, Clone)]
// struct SseMessage {
//     pub message: Vec<String>,
// }

// impl SseMessage {

//     fn data(&mut self, value:&str) -> &mut Self {
//         self.message.push(format!("data:{}\n",value));
//         self
//     }

//     fn event(&mut self, value:&str) -> &mut Self {
//         self.message.push(format!("event:{}\n",value));
//         self
//     }

//     fn id(&mut self, value:&str) -> &mut Self {
//         self.message.push(format!("id:{}\n",value));
//         self
//     }

//     fn retry(&mut self, value:&str) -> &mut Self {
//         self.message.push(format!("retry:{}\n",value));
//         self
//     }
//     fn end(&mut self, value:&str) -> &mut Self {
//         self.message.push(format!("retry:{}\n",value));
//         self
//     }
//     fn collect(&mut self)-> Bytes {
//         self.message.push("\n".into());
//         let data:String = self.message.clone().into_iter().collect();

//         Bytes::from(data)
//     }
// }

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // fn encode_data() {
    //     let msg = encode_msg(Some("hello".into()),None,None,None);

    //     assert_eq!(msg, "data:hello\n\n");
    // }

    // #[test]
    // fn encode_data_event() {
    //     let msg = encode_msg(Some("hello".into()),Some("ping".into()),None,None);

    //     assert_eq!(msg, "data:hello\nevent:ping\n\n");
    // }

    // #[test]
    // fn encode_data_id() {
    //     let msg = encode_msg(Some("hello".into()),None,Some("1234".into()),None);

    //     assert_eq!(msg, "data:hello\nid:1234\n\n");
    // }
}
