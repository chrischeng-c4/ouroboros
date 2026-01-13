//! data-bridge-api: High-performance API framework
//!
//! A Rust-based API framework designed as a FastAPI replacement,
//! following the data-bridge architecture principles:
//! - Rust handles validation, routing, serialization
//! - Python defines contracts via type hints
//! - Two-phase GIL pattern for maximum concurrency

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
pub mod server;
pub mod python_handler;

// OpenTelemetry tracing - only available with "observability" feature
#[cfg(feature = "observability")]
pub mod telemetry;

// Re-exports
pub use router::Router;
pub use handler::{Handler, HandlerMeta};
pub use request::Request;
pub use response::Response;
pub use error::{ApiError, ApiResult};
pub use server::{Server, ServerConfig};
pub use python_handler::PythonHandler;

// Re-export telemetry types when feature is enabled
#[cfg(feature = "observability")]
pub use telemetry::{TelemetryConfig, init_telemetry, shutdown_telemetry};

// Re-export shared HTTP types from data-bridge-common
pub use data_bridge_common::http::{HttpMethod, HttpStatus, HttpResponseLike, HttpRequestLike};
