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
}

impl<const N: usize> From<ytextract::error::Id<N>> for Error {
    fn from(value: ytextract::error::Id<N>) -> Self {
        Self::InvalidId(value.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let err = self.to_string();
        error!("Error: {err}");
        let status = match self {
            Error::YTErr(_) | Error::Scrape(_) | Error::HttpRequest(_) => StatusCode::BAD_GATEWAY,
            Error::InvalidId(_) => StatusCode::BAD_REQUEST,
            Error::CacheError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Response::builder().status(status).body(boxed(err)).unwrap()
    }
}
