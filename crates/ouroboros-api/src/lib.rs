//! ouroboros-api: High-performance API framework
//!
//! A Rust-based API framework designed as a FastAPI replacement,
//! following the ouroboros architecture principles:
//! - Rust handles validation, routing, serialization
//! - Python defines contracts via type hints
//! - Two-phase GIL pattern for maximum concurrency
//!
//! # Architecture
//!
//! The framework uses a two-phase approach to maximize concurrency:
//!
//! ```text
//! Request → [Rust: Parse/Validate] → [Python: Handler] → [Rust: Serialize] → Response
//!               No GIL needed           GIL acquired        No GIL needed
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use ouroboros_api::{Router, Server, ServerConfig, Response};
//! use ouroboros_api::request::SerializableValue;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut router = Router::new();
//!     router.get("/", |_req| async {
//!         Response::json(SerializableValue::String("Hello!".to_string()))
//!     });
//!
//!     let config = ServerConfig::default();
//!     Server::new(config).serve(router).await.unwrap();
//! }
//! ```
//!
//! # Modules
//!
//! ## Core
//! - [`router`]: Radix tree routing with path parameter extraction
//! - [`handler`]: Handler traits and metadata for route handlers
//! - [`request`]: Request abstraction with type-safe parameter access
//! - [`response`]: Response builder supporting JSON, HTML, and more
//! - [`middleware`]: Composable middleware chain with Tower integration
//!
//! ## Validation & Extraction
//! - [`validation`]: Request validation powered by ouroboros-validation
//! - [`extractors`]: Type-safe extractors for Path, Query, Body, Headers
//! - [`error`]: Error types with automatic HTTP status code mapping
//!
//! ## Security
//! - [`security`]: JWT, OAuth2, and API key authentication
//! - [`rate_limit`]: Token bucket and sliding window rate limiting
//! - [`cookies`]: Secure cookie handling with HMAC signing
//!
//! ## Real-time
//! - [`websocket`]: WebSocket connection handling
//! - [`sse`]: Server-Sent Events for push notifications
//!
//! ## Content
//! - [`compression`]: Automatic response compression (gzip, deflate)
//! - [`content_negotiation`]: Accept header parsing and media type matching
//! - [`static_files`]: Static file serving with caching
//! - [`templates`]: Template rendering support
//! - [`upload`]: File upload handling with streaming support
//!
//! ## Lifecycle
//! - [`lifecycle`]: Startup and shutdown hooks
//! - [`background_tasks`]: Background task execution
//!
//! ## Documentation
//! - [`openapi`]: OpenAPI schema generation via utoipa
//!
//! # Feature Flags
//!
//! - **observability**: Enable OpenTelemetry distributed tracing
//! - **bson**: Enable MongoDB BSON type support in validation

pub mod background_tasks;
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
pub mod websocket;
pub mod sse;
pub mod compression;
pub mod content_negotiation;
pub mod cookies;
pub mod lifecycle;
pub mod rate_limit;
pub mod security;
pub mod static_files;
pub mod templates;
pub mod upload;

// OpenTelemetry tracing - only available with "observability" feature
#[cfg(feature = "observability")]
pub mod telemetry;

// Development server with hot reload - only available with "dev" feature
#[cfg(feature = "dev")]
pub mod dev_server;

// Re-exports
pub use background_tasks::{BackgroundTasks, SharedBackgroundTasks, TaskBuilder};
pub use router::Router;
pub use handler::{Handler, HandlerMeta};
pub use request::Request;
pub use response::Response;
pub use error::{ApiError, ApiResult};
pub use server::{Server, ServerConfig};
pub use python_handler::PythonHandler;
pub use sse::{SseEvent, SseStream, SseResponse};
pub use lifecycle::{LifecycleManager, SharedLifecycleManager, StartupError};
pub use static_files::{StaticFiles, StaticFilesConfig};
pub use templates::{Templates, TemplateConfig, Context, ContextValue, SharedTemplates, shared_templates};
pub use upload::{UploadConfig, StreamingUpload, UploadProgress, UploadedFile, MultipartStream, upload_channel};
pub use cookies::{Cookie, CookieJar, CookieSigner, SameSite, ResponseCookies};
pub use security::{JwtConfig, JwtClaims, JwtHandler, JwtAlgorithm, OAuth2PasswordBearer, TokenResponse, ApiKey, ApiKeyLocation};
pub use rate_limit::{RateLimitConfig, RateLimitAlgorithm, RateLimitResult, RateLimiter, SharedRateLimiter, shared_rate_limiter, RateLimitTier, TieredRateLimiter};
pub use compression::{CompressionConfig, CompressionAlgorithm, CompressionLevel, CompressionResult, ResponseCompressor, compress, compress_gzip, compress_deflate};
pub use content_negotiation::{MediaType, AcceptHeader, ContentNegotiator, NegotiationResult, LanguageTag, AcceptLanguage};

// Re-export telemetry types when feature is enabled
#[cfg(feature = "observability")]
pub use telemetry::{TelemetryConfig, init_telemetry, shutdown_telemetry};

// Re-export shared HTTP types from ouroboros-common
pub use ouroboros_common::http::{HttpMethod, HttpStatus, HttpResponseLike, HttpRequestLike};
