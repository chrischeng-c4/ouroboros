//! Common utilities for data-bridge
//!
//! This crate provides shared functionality used across all data-bridge modules.

pub mod error;
pub mod http;

pub use error::{DataBridgeError, Result};
pub use http::{HttpMethod, HttpStatus, HttpRequestLike, HttpResponseLike};
