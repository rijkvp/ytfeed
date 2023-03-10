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
    #[error("YouTube extractor: {0}")]
    YTErr(#[from] ytextract::Error),
    #[error("Invalid id: {0}")]
    InvalidId(String),
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
}

impl<const N: usize> From<ytextract::error::Id<N>> for Error {
    fn from(value: ytextract::error::Id<N>) -> Self {
        Self::InvalidId(value.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, log) = match self {
            // Internal
            Error::YTErr(_) | Error::Scrape(_) | Error::HttpRequest(_) => {
                (StatusCode::BAD_GATEWAY, true)
            }
            Error::CacheError(_) => (StatusCode::INTERNAL_SERVER_ERROR, true),
            // Other
            Error::InvalidId(_) => (StatusCode::BAD_REQUEST, false),
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
