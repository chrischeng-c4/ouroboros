//! Unit tests for postgres module.

use super::conversion::adjust_placeholders;

#[test]
fn test_adjust_placeholders_valid() {
    // Test basic placeholder adjustment
    let sql = "age > $1 AND status = $2";
    let result = adjust_placeholders(sql, 3);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "age > $4 AND status = $5");
}

#[test]
fn test_adjust_placeholders_no_placeholders() {
    // Test SQL with no placeholders
    let sql = "SELECT * FROM users";
    let result = adjust_placeholders(sql, 5);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "SELECT * FROM users");
}

#[test]
fn test_adjust_placeholders_zero_offset() {
    // Test with zero offset (no adjustment)
    let sql = "price < $1";
    let result = adjust_placeholders(sql, 0);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "price < $1");
}

#[test]
fn test_adjust_placeholders_multiple_digits() {
    // Test with multi-digit placeholder numbers
    let sql = "col1 = $10 AND col2 = $15";
    let result = adjust_placeholders(sql, 5);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "col1 = $15 AND col2 = $20");
}

#[test]
fn test_adjust_placeholders_invalid_number() {
    // Test with invalid placeholder number (too large for usize on some systems)
    // This is theoretical but good to test error handling
    let sql = "value = $99999999999999999999999999999";
    let result = adjust_placeholders(sql, 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid placeholder number"));
}

// Note: Panic boundary tests for safe_call and future_into_py require Python runtime
// and cannot be run as standard Rust unit tests due to PyO3 dependencies.
//
// These functions should be tested via:
// 1. Python integration tests that trigger panic scenarios
// 2. Manual verification during development
// 3. Code review to ensure proper panic-catching patterns
//
// See PANIC_SAFETY.md for testing guidelines and usage examples.
