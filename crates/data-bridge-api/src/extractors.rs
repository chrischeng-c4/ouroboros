//! Request data extractors
//!
//! Extractors pull typed data from requests (path params, query params, body, etc.).

use crate::request::Request;
use crate::error::{ApiResult, ApiError};
use serde::de::DeserializeOwned;

/// Path parameter extractor
pub struct Path<T> {
    pub inner: T,
}

impl<T> Path<T>
where
    T: DeserializeOwned,
{
    /// Extract path parameters from request
    pub fn from_request(_req: &Request) -> ApiResult<Self> {
        // TODO: Implement extraction logic
        Err(ApiError::Internal("Path extraction not implemented".to_string()))
    }
}

/// Query parameter extractor
pub struct Query<T> {
    pub inner: T,
}

impl<T> Query<T>
where
    T: DeserializeOwned,
{
    /// Extract query parameters from request
    pub fn from_request(_req: &Request) -> ApiResult<Self> {
        // TODO: Implement extraction logic
        Err(ApiError::Internal("Query extraction not implemented".to_string()))
    }
}

/// JSON body extractor
pub struct Json<T> {
    pub inner: T,
}

impl<T> Json<T>
where
    T: DeserializeOwned,
{
    /// Extract JSON body from request
    pub fn from_request(req: &Request) -> ApiResult<Self> {
        let json_value = req.body_json().ok_or_else(|| {
            ApiError::BadRequest("Missing request body".to_string())
        })?;

        let inner = serde_json::from_value(json_value).map_err(|e| {
            ApiError::BadRequest(format!("Invalid JSON: {}", e))
        })?;

        Ok(Json { inner })
    }
}

/// Header extractor
pub struct Headers {
    inner: std::collections::HashMap<String, String>,
}

impl Headers {
    /// Extract headers from request
    pub fn from_request(req: &Request) -> ApiResult<Self> {
        Ok(Headers {
            inner: req.inner.headers.clone(),
        })
    }

    /// Get a header value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.inner.get(key)
    }
}
