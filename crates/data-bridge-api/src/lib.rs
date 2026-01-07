//! data-bridge-api: High-performance API framework
//!
//! A Rust-based API framework designed as a FastAPI replacement,
//! following the data-bridge architecture principles:
//! - Rust handles validation, routing, serialization
//! - Python defines contracts via type hints
//! - Two-phase GIL pattern for maximum concurrency

pub mod app;
pub mod router;
pub mod handler;
pub mod request;
pub mod response;
pub mod middleware;
pub mod dependency;
pub mod extractors;
pub mod error;
pub mod validation;
pub mod openapi;

// Re-exports
pub use app::App;
pub use router::Router;
pub use handler::{Handler, HandlerMeta};
pub use request::Request;
pub use response::Response;
pub use error::{ApiError, ApiResult};

// Re-export shared HTTP types from data-bridge-common
pub use data_bridge_common::http::{HttpMethod, HttpStatus, HttpResponseLike, HttpRequestLike};
