//! Validation constraints for different types
//!
//! This module defines constraints for validating strings, numbers, lists, and object fields.

use crate::types::{TypeDescriptor, Value};

// ============================================================================
// String Constraints
// ============================================================================

/// Constraints for string validation
#[derive(Debug, Clone, Default)]
pub struct StringConstraints {
    /// Minimum length (in characters, not bytes)
    pub min_length: Option<usize>,
    /// Maximum length (in characters, not bytes)
    pub max_length: Option<usize>,
    /// Regex pattern (compiled at validation time)
    pub pattern: Option<String>,
    /// Predefined format validator
    pub format: Option<StringFormat>,
}

/// Predefined string format validators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringFormat {
    /// Email address format
    Email,
    /// URL format (http/https)
    Url,
    /// UUID format (v4)
    Uuid,
    /// ISO 8601 DateTime format
    DateTime,
    /// Date format (YYYY-MM-DD)
    Date,
    /// Time format (HH:MM:SS)
    Time,
}

// ============================================================================
// Numeric Constraints
// ============================================================================

/// Constraints for numeric validation (generic over i64 and f64)
#[derive(Debug, Clone, Default)]
pub struct NumericConstraints<T> {
    /// Minimum value (inclusive)
    pub minimum: Option<T>,
    /// Maximum value (inclusive)
    pub maximum: Option<T>,
    /// Minimum value (exclusive)
    pub exclusive_minimum: Option<T>,
    /// Maximum value (exclusive)
    pub exclusive_maximum: Option<T>,
    /// Value must be a multiple of this number
    pub multiple_of: Option<T>,
}

// ============================================================================
// List Constraints
// ============================================================================

/// Constraints for list/array validation
#[derive(Debug, Clone, Default)]
pub struct ListConstraints {
    /// Minimum number of items
    pub min_items: Option<usize>,
    /// Maximum number of items
    pub max_items: Option<usize>,
    /// Whether all items must be unique
    pub unique_items: bool,
}

// ============================================================================
// Field Descriptor (for Object validation)
// ============================================================================

/// Field descriptor for object/dictionary validation
#[derive(Debug, Clone)]
pub struct FieldDescriptor {
    /// Field name
    pub name: String,
    /// Type descriptor for this field
    pub type_desc: TypeDescriptor,
    /// Whether this field is required
    pub required: bool,
    /// Default value if field is missing
    pub default: Option<Value>,
    /// Optional description for documentation
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_constraints_default() {
        let constraints = StringConstraints::default();
        assert!(constraints.min_length.is_none());
        assert!(constraints.max_length.is_none());
        assert!(constraints.pattern.is_none());
        assert!(constraints.format.is_none());
    }

    #[test]
    fn test_numeric_constraints_default() {
        let constraints: NumericConstraints<i64> = NumericConstraints::default();
        assert!(constraints.minimum.is_none());
        assert!(constraints.maximum.is_none());
        assert!(constraints.exclusive_minimum.is_none());
        assert!(constraints.exclusive_maximum.is_none());
        assert!(constraints.multiple_of.is_none());
    }

    #[test]
    fn test_list_constraints_default() {
        let constraints = ListConstraints::default();
        assert!(constraints.min_items.is_none());
        assert!(constraints.max_items.is_none());
        assert!(!constraints.unique_items);
    }

    #[test]
    fn test_string_format_equality() {
        assert_eq!(StringFormat::Email, StringFormat::Email);
        assert_ne!(StringFormat::Email, StringFormat::Url);
    }
}
