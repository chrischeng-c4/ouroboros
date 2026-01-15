//! ouroboros-http: High-performance async HTTP client
//!
//! A Rust HTTP client library designed for integration with Python via PyO3.
//! Provides connection pooling, automatic latency measurement, and async operations.
//!
//! # Architecture
//!
//! This crate provides the core HTTP functionality:
//! - `HttpClient`: Connection-pooled async HTTP client
//! - `Request`: Request builder with headers, body, auth
//! - `Response`: Response wrapper with latency measurement
//!
//! PyO3 bindings are in the `ouroboros` crate.

pub mod client;
pub mod config;
pub mod error;
pub mod request;
pub mod response;

pub use client::HttpClient;
pub use config::HttpClientConfig;
pub use error::{HttpError, HttpResult};
pub use request::{HttpMethod, RequestBuilder};
pub use response::HttpResponse;

// Re-export shared HTTP types from ouroboros-common
pub use ouroboros_common::http::{HttpStatus, HttpResponseLike, HttpRequestLike};
