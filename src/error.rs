use axum::{response::{IntoResponse, Response}, http::StatusCode, Json};
use serde::Serialize;
use serde_json::json;
// use std::convert::Infallible;
use thiserror::Error;
// use hyper::{http::StatusCode, Rejection, Reply};

#[derive(Error, Debug)]
pub enum Error {
    #[error("wrong credentials")]
    _WrongCredentialsError,
    #[error("InvalidToken")]
    InvalidToken,
    #[error("jwt token creation error")]
    JWTTokenCreationError,
    #[error("no auth header")]
    NoAuthHeaderError,
    #[error("invalid auth header")]
    InvalidAuthHeaderError,
    #[error("no permission")]
    _NoPermissionError,

    #[error("broadcaster lock error")]
    BroadcasterLockError,

    #[error("error while sending message to broadcaster")]
    EventSendMessage,
}

// #[derive(Error, Debug)]
// pub enum AuthError {
//     #[error("wrong credentials")]
//     _WrongCredentialsError,
//     #[error("jwt token not valid")]
//     JWTTokenError,
//     #[error("jwt token creation error")]
//     JWTTokenCreationError,
//     #[error("no auth header")]
//     NoAuthHeaderError,
//     #[error("invalid auth header")]
//     InvalidAuthHeaderError,
//     #[error("no permission")]
//     _NoPermissionError,
// }

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}

// impl warp::reject::Reject for Error {}

// pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
//     let (code, message) = if err.is_not_found() {
//         (StatusCode::NOT_FOUND, "Not Found".to_string())
//     } else if let Some(e) = err.find::<Error>() {
//         match e {
//             Error::WrongCredentialsError => (StatusCode::FORBIDDEN, e.to_string()),
//             Error::NoPermissionError => (StatusCode::UNAUTHORIZED, e.to_string()),
//             Error::JWTTokenError => (StatusCode::UNAUTHORIZED, e.to_string()),
//             Error::JWTTokenCreationError => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "Internal Server Error".to_string(),
//             ),
//             _ => (StatusCode::BAD_REQUEST, e.to_string()),
//         }
//     } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
//         (
//             StatusCode::METHOD_NOT_ALLOWED,
//             "Method Not Allowed".to_string(),
//         )
//     } else {
//         eprintln!("unhandled error: {:?}", err);
//         (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "Internal Server Error".to_string(),
//         )
//     };

//     let json = warp::reply::json(&ErrorResponse {
//         status: code.to_string(),
//         message,
//     });

//     Ok(warp::reply::with_status(json, code))
// }

/// Our app's top level error type.
pub enum AppError {
    /// Something went wrong when calling the user repo.
    UserRepo(UserRepoError),
}

/// This makes it possible to use `?` to automatically convert a `UserRepoError`
/// into an `AppError`.
impl From<UserRepoError> for AppError {
    fn from(inner: UserRepoError) -> Self {
        AppError::UserRepo(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::UserRepo(UserRepoError::NotFound) => {
                (StatusCode::NOT_FOUND, "User not found")
            }
            AppError::UserRepo(UserRepoError::InvalidUsername) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "Invalid username")
            }
            AppError::UserRepo(UserRepoError::InvalidFunction) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "Invalid function")
            }
            AppError::UserRepo(UserRepoError::FailFunctionExecution) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "Fail function execution")
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

/// Errors that can happen when using the user repo.
// #[derive(Debug)]
#[derive(Error, Debug)]
pub enum UserRepoError {
    #[error("error while sending message to broadcaster")]
    NotFound,
    #[error("error while sending message to broadcaster")]
    InvalidUsername,

    #[error("error while sending message to broadcaster")]
    InvalidFunction,
    #[error("error while sending message to broadcaster")]
    FailFunctionExecution,
}