//! Format validation for common string patterns
//!
//! This module provides pre-compiled regex validators for common formats like
//! email, URL, UUID, and various date/time formats.

use once_cell::sync::Lazy;
use regex::Regex;

// ============================================================================
// Pre-compiled Regex Patterns
// ============================================================================

/// Email regex pattern (RFC 5322 simplified)
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

/// URL regex pattern (http/https)
static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap()
});

/// UUID regex pattern (v4)
static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$").unwrap()
});

/// ISO 8601 DateTime regex pattern
static DATETIME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$").unwrap()
});

/// Date regex pattern (YYYY-MM-DD)
static DATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap()
});

/// Time regex pattern (HH:MM:SS with optional fractional seconds)
/// Note: This validates format only, not valid time ranges
static TIME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)(\.\d{1,9})?$").unwrap()
});

// ============================================================================
// Format Validators
// ============================================================================

/// Validate email format
///
/// # Example
/// ```
/// use ouroboros_validation::formats::validate_email;
///
/// assert!(validate_email("user@example.com"));
/// assert!(!validate_email("invalid-email"));
/// ```
pub fn validate_email(value: &str) -> bool {
    EMAIL_REGEX.is_match(value)
}

/// Validate URL format (http/https)
///
/// # Example
/// ```
/// use ouroboros_validation::formats::validate_url;
///
/// assert!(validate_url("https://example.com"));
/// assert!(validate_url("http://localhost:8080/path"));
/// assert!(!validate_url("ftp://example.com"));
/// ```
pub fn validate_url(value: &str) -> bool {
    URL_REGEX.is_match(value)
}

/// Validate UUID format (v4)
///
/// # Example
/// ```
/// use ouroboros_validation::formats::validate_uuid;
///
/// assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000"));
/// assert!(!validate_uuid("not-a-uuid"));
/// ```
pub fn validate_uuid(value: &str) -> bool {
    UUID_REGEX.is_match(value)
}

/// Validate ISO 8601 DateTime format
///
/// # Example
/// ```
/// use ouroboros_validation::formats::validate_datetime;
///
/// assert!(validate_datetime("2024-01-19T12:00:00Z"));
/// assert!(validate_datetime("2024-01-19T12:00:00.123456789+08:00"));
/// assert!(!validate_datetime("2024-01-19 12:00:00"));
/// ```
pub fn validate_datetime(value: &str) -> bool {
    DATETIME_REGEX.is_match(value)
}

/// Validate date format (YYYY-MM-DD)
///
/// # Example
/// ```
/// use ouroboros_validation::formats::validate_date;
///
/// assert!(validate_date("2024-01-19"));
/// assert!(!validate_date("01/19/2024"));
/// ```
pub fn validate_date(value: &str) -> bool {
    DATE_REGEX.is_match(value)
}

/// Validate time format (HH:MM:SS with optional fractional seconds)
///
/// # Example
/// ```
/// use ouroboros_validation::formats::validate_time;
///
/// assert!(validate_time("12:00:00"));
/// assert!(validate_time("23:59:59.999"));
/// assert!(!validate_time("25:00:00"));
/// ```
pub fn validate_time(value: &str) -> bool {
    TIME_REGEX.is_match(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        // Valid emails
        assert!(validate_email("user@example.com"));
        assert!(validate_email("test.user+tag@subdomain.example.co.uk"));
        assert!(validate_email("admin@localhost.local"));

        // Invalid emails
        assert!(!validate_email("invalid-email"));
        assert!(!validate_email("@example.com"));
        assert!(!validate_email("user@"));
        assert!(!validate_email("user@.com"));
        assert!(!validate_email("user@example"));
    }

    #[test]
    fn test_url_validation() {
        // Valid URLs
        assert!(validate_url("https://example.com"));
        assert!(validate_url("http://localhost:8080"));
        assert!(validate_url("https://sub.domain.example.com/path?query=value"));

        // Invalid URLs
        assert!(!validate_url("ftp://example.com"));
        assert!(!validate_url("not-a-url"));
        assert!(!validate_url("://example.com"));
    }

    #[test]
    fn test_uuid_validation() {
        // Valid UUIDs (v4)
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(validate_uuid("6ba7b810-9dad-41d1-80b4-00c04fd430c8"));

        // Invalid UUIDs
        assert!(!validate_uuid("not-a-uuid"));
        assert!(!validate_uuid("550e8400-e29b-11d4-a716-446655440000")); // Not v4
        assert!(!validate_uuid("550e8400e29b41d4a716446655440000")); // Missing hyphens
    }

    #[test]
    fn test_datetime_validation() {
        // Valid DateTimes
        assert!(validate_datetime("2024-01-19T12:00:00Z"));
        assert!(validate_datetime("2024-01-19T12:00:00+08:00"));
        assert!(validate_datetime("2024-01-19T12:00:00.123456789Z"));

        // Invalid DateTimes
        assert!(!validate_datetime("2024-01-19 12:00:00"));
        assert!(!validate_datetime("2024-01-19T12:00:00"));
        assert!(!validate_datetime("01/19/2024 12:00:00"));
    }

    #[test]
    fn test_date_validation() {
        // Valid dates
        assert!(validate_date("2024-01-19"));
        assert!(validate_date("2000-12-31"));

        // Invalid dates
        assert!(!validate_date("01/19/2024"));
        assert!(!validate_date("2024-1-19"));
        assert!(!validate_date("24-01-19"));
    }

    #[test]
    fn test_time_validation() {
        // Valid times
        assert!(validate_time("12:00:00"));
        assert!(validate_time("23:59:59"));
        assert!(validate_time("00:00:00.123456789"));

        // Invalid times
        assert!(!validate_time("25:00:00"));
        assert!(!validate_time("12:00"));
        assert!(!validate_time("12:00:00 PM"));
    }
}
