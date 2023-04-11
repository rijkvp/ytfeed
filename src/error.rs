use crate::cache::CacheError;
use axum::{
    body::boxed,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed HTTP request: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("Scraping: {0}")]
    Scrape(String),
    #[error("Cache: {0}")]
    CacheError(#[from] CacheError),
    #[error("Channel '{0}' not found")]
    ChannelNotFound(String),
    #[error("Failed to extract info from '{0}'")]
    Extraction(String),
    #[error("JSON parse: {0}")]
    Json(#[from] serde_json::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, log) = match self {
            // Internal
            Error::Json(_) | Error::Scrape(_) | Error::HttpRequest(_) => (StatusCode::BAD_GATEWAY, true),
            Error::CacheError(_) => (StatusCode::INTERNAL_SERVER_ERROR, true),
            // Other
            Error::ChannelNotFound(_) => (StatusCode::NOT_FOUND, false),
            Error::Extraction(_) => (StatusCode::BAD_GATEWAY, false),
        };
        let msg = self.to_string();
        if log {
            error!("{msg}");
            Response::builder()
                .status(status)
                .body(boxed(String::with_capacity(0)))
                .unwrap()
        } else {
            Response::builder().status(status).body(boxed(msg)).unwrap()
        }
    }
}
