use crate::error::Error;
use anyhow::anyhow;
use anyhow::Result;
use axum::headers::authorization::Bearer;
use chrono::prelude::*;
use chrono::Duration;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
// const BEARER: &str = "Bearer ";
// pub const JWT_SECRET: &[u8] = b"!ChangeMe!";
static KEYS: Lazy<Keys> = Lazy::new(|| {
    // let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let secret = "!ChangeMe!";
    Keys::new(secret.as_bytes())
});
struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublisherMercure {
    pub publish: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublisherClaims {
    pub sub: String,
    pub mercure: PublisherMercure,
    pub exp: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubscriberMercure {
    pub subscribe: Vec<String>,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubscriberClaims {
    pub sub: String,
    pub mercure: SubscriberMercure,
    pub exp: usize,
}

// pub fn with_auth(role: Role) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
//     headers_cloned()
//         .map(move |headers: HeaderMap<HeaderValue>| (role.clone(), headers))
//         .and_then(authorize)
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Claims<T, E> {
    Ok(T),
    Err(E),
}

pub fn create_jwt<T: Serialize>(claims: &T) -> Result<String> {
    let _expiration = Utc::now()
        // .checked_add_signed(chrono::Duration::seconds(60))
        .checked_add_signed(Duration::hours(99999))
        .expect("valid timestamp")
        .timestamp();

    // let header = Header::new(Algorithm::HS512);
    let header = Header::default();
    encode(&header, &claims, &KEYS.encoding)
        .map_err(|_| anyhow!(Error::JWTTokenCreationError))
}

pub async fn authorize_publisher(bearer:Option<Bearer>) -> Result<PublisherMercure> {
    if let Some(bearer) = bearer {
        let token_data = decode::<PublisherClaims>(bearer.token(), &KEYS.decoding, &Validation::default())
        .map_err(|_| Error::InvalidToken)?;
    
        return Ok(token_data.claims.mercure);
    } else {
        return Err(anyhow!(Error::NoAuthHeaderError));
    }        
}
pub async fn authorize_subscriber(bearer:Option<Bearer>) -> Result<SubscriberMercure> {
    if let Some(bearer) = bearer {
        let token_data = decode::<SubscriberClaims>(bearer.token(), &KEYS.decoding, &Validation::default())
        .map_err(|_| Error::InvalidToken)?;    

        return Ok(token_data.claims.mercure);
    } else {
        return Err(anyhow!(Error::NoAuthHeaderError));
    }
}

// pub async fn authorize_publisher(headers: HeaderMap<HeaderValue>) -> Result<PublisherMercure> {
//     match jwt_from_header(&headers) {
//         Ok(jwt) => {
//             let decoded = decode::<PublisherClaims>(
//                 &jwt,
//                 &KEYS.decoding,
//                 &Validation::new(Algorithm::HS512),
//             )
//             .map_err(|_| anyhow!(Error::InvalidToken))?;

//             // if role == Role::Admin && Role::from_str(&decoded.claims.role) != Role::Admin {
//             //     return Err(anyhow!(Error::NoPermissionError));
//             // }

//             Ok(decoded.claims.mercure)
//         }
//         Err(e) => return Err(anyhow!(e)),
//     }
// }

// pub async fn authorize_subscriber(headers: HeaderMap<HeaderValue>) -> Result<SubscriberMercure> {
//     match jwt_from_header(&headers) {
//         Ok(jwt) => {
//             let decoded = decode::<SubscriberClaims>(
//                 &jwt,
//                 &KEYS.decoding,
//                 &Validation::new(Algorithm::HS512),
//             )
//             .map_err(|_| anyhow!(Error::InvalidToken))?;

//             // if role == Role::Admin && Role::from_str(&decoded.claims.role) != Role::Admin {
//             //     return Err(anyhow!(Error::NoPermissionError));
//             // }

//             Ok(decoded.claims.mercure)
//         }
//         Err(e) => return Err(anyhow!(e)),
//     }
// }

// pub fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String> {
//     let header = match headers.get(AUTHORIZATION) {
//         Some(v) => v,
//         None => return Err(anyhow!(Error::NoAuthHeaderError)),
//     };
//     let auth_header = match std::str::from_utf8(header.as_bytes()) {
//         Ok(v) => v,
//         Err(_) => return Err(anyhow!(Error::NoAuthHeaderError)),
//     };
//     if !auth_header.starts_with(BEARER) {
//         return Err(anyhow!(Error::InvalidAuthHeaderError));
//     }
//     Ok(auth_header.trim_start_matches(BEARER).to_owned())
// }
