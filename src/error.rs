use crate::cache::CacheError;
use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed HTTP request: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("Scraping: {0}")]
    Scrape(String),
    #[error("Cache: {0}")]
    Cache(#[from] CacheError),
    #[error("Channel '{0}' not found")]
    ChannelNotFound(String),
    #[error("Failed to proxy feed '{0}'")]
    Proxy(String),
    #[error("JSON parse: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Feed parse: {0}")]
    Feed(#[from] atom_syndication::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, log) = match self {
            // Internal
            Error::Json(_) | Error::Feed(_) | Error::Scrape(_) | Error::HttpRequest(_) => {
                (StatusCode::BAD_GATEWAY, true)
            }
            Error::Cache(_) => (StatusCode::INTERNAL_SERVER_ERROR, true),
            // Other
            Error::ChannelNotFound(_) => (StatusCode::NOT_FOUND, false),
            Error::Proxy(_) => (StatusCode::BAD_GATEWAY, false),
        };
        let msg = self.to_string();
        if log {
            tracing::error!("{msg}");
            Response::builder()
                .status(status)
                .body(Body::from(String::with_capacity(0)))
                .unwrap()
        } else {
            Response::builder()
                .status(status)
                .body(Body::from(msg))
                .unwrap()
        }
    }
}
