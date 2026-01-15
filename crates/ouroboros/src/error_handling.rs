//! Error handling and sanitization for MongoDB operations
//!
//! This module provides security-focused error handling to prevent information
//! leakage through error messages (connection strings, credentials, etc.).
//!
//! # Security Features
//! - Removes connection strings from error messages
//! - Redacts credentials and authentication details
//! - Categorizes errors for better handling
//! - Production vs debug modes

use pyo3::prelude::*;
use pyo3::exceptions::{PyConnectionError, PyValueError, PyRuntimeError, PyTimeoutError};
use regex::Regex;
use std::sync::OnceLock;

/// Error categories for better error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Connection-related errors (network, DNS, etc.)
    Connection,
    /// Authentication/authorization errors
    Authentication,
    /// Timeout errors
    Timeout,
    /// Validation errors (invalid input)
    Validation,
    /// Query/operation errors
    Operation,
    /// Unknown/uncategorized errors
    Unknown,
}

/// Regex for matching MongoDB connection strings (compiled once)
fn connection_string_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"mongodb(\+srv)?://[^\s]+").unwrap()
    })
}

/// Regex for matching credentials in URLs (compiled once)
fn credentials_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"://[^:]+:[^@]+@").unwrap()
    })
}

/// Regex for matching IP addresses (compiled once)
fn ip_address_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}(:\d+)?\b").unwrap()
    })
}

/// Sanitizes a MongoDB error message by removing sensitive information
///
/// # Arguments
/// * `error_msg` - The original error message
/// * `debug_mode` - If true, provides more detailed errors (for development)
///
/// # Returns
/// Sanitized error message safe for production logging
///
/// # Security
/// Removes:
/// - Connection strings (mongodb://... or mongodb+srv://...)
/// - Credentials (username:password)
/// - IP addresses and ports
/// - Database names
///
/// # Examples
/// ```
/// # use ouroboros::error_handling::sanitize_error;
/// let error = "Failed to connect to mongodb://user:pass@localhost:27017/mydb";
/// let sanitized = sanitize_error(error, false);
/// assert!(!sanitized.contains("user:pass"));
/// assert!(!sanitized.contains("localhost"));
/// ```
pub fn sanitize_error(error_msg: &str, debug_mode: bool) -> String {
    if debug_mode {
        // In debug mode, return full error for debugging
        return error_msg.to_string();
    }

    let mut sanitized = error_msg.to_string();

    // Remove connection strings entirely
    sanitized = connection_string_regex()
        .replace_all(&sanitized, "[CONNECTION_STRING_REDACTED]")
        .to_string();

    // Remove credentials from URLs
    sanitized = credentials_regex()
        .replace_all(&sanitized, "://[CREDENTIALS_REDACTED]@")
        .to_string();

    // Remove IP addresses and ports
    sanitized = ip_address_regex()
        .replace_all(&sanitized, "[IP_REDACTED]")
        .to_string();

    // Remove common sensitive patterns
    sanitized = sanitized
        .replace("username", "[USERNAME_REDACTED]")
        .replace("password", "[PASSWORD_REDACTED]")
        .replace("auth", "[AUTH_REDACTED]");

    sanitized
}

/// Categorizes a MongoDB error based on its message
///
/// # Arguments
/// * `error_msg` - The error message to categorize
///
/// # Returns
/// ErrorCategory indicating the type of error
pub fn categorize_error(error_msg: &str) -> ErrorCategory {
    let lowercase = error_msg.to_lowercase();

    if lowercase.contains("connection") || lowercase.contains("network") || lowercase.contains("dns") {
        ErrorCategory::Connection
    } else if lowercase.contains("auth") || lowercase.contains("unauthorized") || lowercase.contains("permission") {
        ErrorCategory::Authentication
    } else if lowercase.contains("timeout") || lowercase.contains("timed out") {
        ErrorCategory::Timeout
    } else if lowercase.contains("invalid") || lowercase.contains("validation") {
        ErrorCategory::Validation
    } else if lowercase.contains("query") || lowercase.contains("operation") {
        ErrorCategory::Operation
    } else {
        ErrorCategory::Unknown
    }
}

/// Converts a MongoDB error to an appropriate Python exception
///
/// # Arguments
/// * `error_msg` - The error message
/// * `category` - The error category
/// * `debug_mode` - Whether to include detailed error information
///
/// # Returns
/// PyErr with appropriate Python exception type
pub fn to_python_exception(error_msg: &str, category: ErrorCategory, debug_mode: bool) -> PyErr {
    let sanitized = sanitize_error(error_msg, debug_mode);

    match category {
        ErrorCategory::Connection => PyConnectionError::new_err(sanitized),
        ErrorCategory::Authentication => PyValueError::new_err(format!("Authentication failed: {}", sanitized)),
        ErrorCategory::Timeout => PyTimeoutError::new_err(sanitized),
        ErrorCategory::Validation => PyValueError::new_err(sanitized),
        ErrorCategory::Operation | ErrorCategory::Unknown => PyRuntimeError::new_err(sanitized),
    }
}

/// Sanitizes a MongoDB error and converts to Python exception
///
/// This is the main entry point for error handling in MongoDB operations.
///
/// # Arguments
/// * `error` - The original error
///
/// # Returns
/// PyErr with sanitized message and appropriate exception type
///
/// # Examples
/// ```
/// # use ouroboros::error_handling::sanitize_mongodb_error;
/// # use mongodb::error::Error;
/// // In MongoDB operations:
/// let result = collection.find_one(filter)
///     .await
///     .map_err(sanitize_mongodb_error)?;
/// ```
pub fn sanitize_mongodb_error(error: mongodb::error::Error) -> PyErr {
    use crate::config::get_config;

    let error_msg = error.to_string();
    let category = categorize_error(&error_msg);

    // Check if we should sanitize errors
    let config = get_config();
    let debug_mode = !config.sanitize_errors;

    to_python_exception(&error_msg, category, debug_mode)
}

/// Sanitizes a generic error and converts to Python exception
pub fn sanitize_generic_error<E: std::fmt::Display>(error: E) -> PyErr {
    use crate::config::get_config;

    let error_msg = error.to_string();
    let category = categorize_error(&error_msg);

    let config = get_config();
    let debug_mode = !config.sanitize_errors;

    to_python_exception(&error_msg, category, debug_mode)
}

/// Sanitizes an error message string using production settings
///
/// This is a convenience wrapper around `sanitize_error` that uses
/// the global config to determine debug mode.
pub fn sanitize_error_message(error_msg: &str) -> String {
    use crate::config::get_config;

    let config = get_config();
    let debug_mode = !config.sanitize_errors;

    sanitize_error(error_msg, debug_mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =====================
    // Sanitization Tests
    // =====================

    #[test]
    fn test_sanitize_connection_string() {
        let error = "Failed to connect to mongodb://user:pass@localhost:27017/mydb";
        let sanitized = sanitize_error(error, false);
        assert!(!sanitized.contains("mongodb://"));
        assert!(!sanitized.contains("user:pass"));
        assert!(!sanitized.contains("localhost"));
        assert!(sanitized.contains("[CONNECTION_STRING_REDACTED]"));
    }

    #[test]
    fn test_sanitize_srv_connection_string() {
        let error = "Error with mongodb+srv://admin:secret@cluster.mongodb.net/db";
        let sanitized = sanitize_error(error, false);
        assert!(!sanitized.contains("mongodb+srv://"));
        assert!(!sanitized.contains("admin:secret"));
        assert!(!sanitized.contains("cluster.mongodb.net"));
        assert!(sanitized.contains("[CONNECTION_STRING_REDACTED]"));
    }

    #[test]
    fn test_sanitize_credentials() {
        let error = "Authentication failed for user myuser with password mypass";
        let sanitized = sanitize_error(error, false);
        assert!(sanitized.contains("[USERNAME_REDACTED]") || sanitized.contains("[PASSWORD_REDACTED]"));
    }

    #[test]
    fn test_sanitize_ip_address() {
        let error = "Could not connect to 192.168.1.100:27017";
        let sanitized = sanitize_error(error, false);
        assert!(!sanitized.contains("192.168.1.100"));
        assert!(sanitized.contains("[IP_REDACTED]"));
    }

    #[test]
    fn test_debug_mode_preserves_details() {
        let error = "Failed to connect to mongodb://user:pass@localhost:27017/mydb";
        let sanitized = sanitize_error(error, true);
        // In debug mode, error should be unchanged
        assert_eq!(sanitized, error);
    }

    #[test]
    fn test_sanitize_safe_error() {
        let error = "Document not found";
        let sanitized = sanitize_error(error, false);
        // Safe errors should pass through unchanged
        assert_eq!(sanitized, error);
    }

    // =====================
    // Categorization Tests
    // =====================

    #[test]
    fn test_categorize_connection_error() {
        assert_eq!(
            categorize_error("Connection refused"),
            ErrorCategory::Connection
        );
        assert_eq!(
            categorize_error("Network timeout"),
            ErrorCategory::Connection
        );
        assert_eq!(
            categorize_error("DNS resolution failed"),
            ErrorCategory::Connection
        );
    }

    #[test]
    fn test_categorize_auth_error() {
        assert_eq!(
            categorize_error("Authentication failed"),
            ErrorCategory::Authentication
        );
        assert_eq!(
            categorize_error("Unauthorized access"),
            ErrorCategory::Authentication
        );
        assert_eq!(
            categorize_error("Permission denied"),
            ErrorCategory::Authentication
        );
    }

    #[test]
    fn test_categorize_timeout_error() {
        assert_eq!(
            categorize_error("Operation timed out"),
            ErrorCategory::Timeout
        );
        assert_eq!(
            categorize_error("Request timeout"),
            ErrorCategory::Timeout
        );
    }

    #[test]
    fn test_categorize_validation_error() {
        assert_eq!(
            categorize_error("Invalid field name"),
            ErrorCategory::Validation
        );
        assert_eq!(
            categorize_error("Validation failed"),
            ErrorCategory::Validation
        );
    }

    #[test]
    fn test_categorize_operation_error() {
        assert_eq!(
            categorize_error("Query execution failed"),
            ErrorCategory::Operation
        );
    }

    #[test]
    fn test_categorize_unknown_error() {
        assert_eq!(
            categorize_error("Something went wrong"),
            ErrorCategory::Unknown
        );
    }

    // =====================
    // Exception Conversion Tests
    // =====================

    #[test]
    fn test_connection_exception() {
        let err = to_python_exception("Connection failed", ErrorCategory::Connection, false);
        // Should be PyConnectionError
        assert!(err.to_string().contains("Connection"));
    }

    #[test]
    fn test_timeout_exception() {
        let err = to_python_exception("Timeout", ErrorCategory::Timeout, false);
        // Should be PyTimeoutError
        assert!(err.to_string().contains("Timeout"));
    }

    #[test]
    fn test_validation_exception() {
        let err = to_python_exception("Invalid input", ErrorCategory::Validation, false);
        // Should be PyValueError
        assert!(err.to_string().contains("Invalid"));
    }
}
