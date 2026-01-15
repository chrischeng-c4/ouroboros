//! Common utilities for ouroboros
//!
//! This crate provides shared functionality used across all ouroboros modules.

pub mod error;
pub mod http;

pub use error::{DataBridgeError, Result};
pub use http::{HttpMethod, HttpStatus, HttpRequestLike, HttpResponseLike};
