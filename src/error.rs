use crate::cache::CacheError;
use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed HTTP request: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("scraping: {0}")]
    Scrape(&'static str),
    #[error("cache: {0}")]
    Cache(#[from] CacheError),
    #[error("channel '{0}' not found")]
    ChannelNotFound(String),
    #[error("Failed to proxy feed '{0}'")]
    Proxy(String),
    #[error("JSON parse: {0}")]
    Json(#[from] serde_json::Error),
    #[error("url encode: {0}")]
    UrlEncode(#[from] serde_html_form::ser::Error),
    #[error("feed parse: {0}")]
    Feed(#[from] atom_syndication::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, log) = match self {
            // Internal
            Error::Json(_) | Error::Feed(_) | Error::HttpRequest(_) => {
                (StatusCode::BAD_GATEWAY, true)
            }
            Error::Scrape(_) | Error::Cache(_) => (StatusCode::INTERNAL_SERVER_ERROR, true),
            Error::UrlEncode(_) => (StatusCode::INTERNAL_SERVER_ERROR, true),
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
