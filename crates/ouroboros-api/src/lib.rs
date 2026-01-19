//! ouroboros-api: High-performance API framework
//!
//! A Rust-based API framework designed as a FastAPI replacement,
//! following the ouroboros architecture principles:
//! - Rust handles validation, routing, serialization
//! - Python defines contracts via type hints
//! - Two-phase GIL pattern for maximum concurrency

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

// Re-export telemetry types when feature is enabled
#[cfg(feature = "observability")]
pub use telemetry::{TelemetryConfig, init_telemetry, shutdown_telemetry};

// Re-export shared HTTP types from ouroboros-common
pub use ouroboros_common::http::{HttpMethod, HttpStatus, HttpResponseLike, HttpRequestLike};
