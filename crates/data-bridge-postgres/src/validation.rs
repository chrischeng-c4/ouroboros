//! Input validation for foreign key references.
//!
//! This module provides validation functions for foreign key references
//! to prevent SQL injection and ensure proper formatting.

use crate::Result;

/// Validates a PostgreSQL identifier (table or column name).
///
/// Ensures the identifier:
/// - Is not empty
/// - Contains only alphanumeric characters, underscores, and dollar signs
/// - Does not start with a digit
/// - Is not a PostgreSQL reserved keyword (basic check)
///
/// # Arguments
///
/// * `identifier` - The identifier to validate
/// * `identifier_type` - Description of what type of identifier (for error messages)
///
/// # Returns
///
/// Ok(()) if valid, Err with descriptive message if invalid
fn validate_identifier(identifier: &str, identifier_type: &str) -> Result<()> {
    if identifier.is_empty() {
        return Err(crate::DataBridgeError::Query(format!(
            "{} cannot be empty",
            identifier_type
        )));
    }

    // Check first character
    let first_char = identifier.chars().next().unwrap();
    if first_char.is_ascii_digit() {
        return Err(crate::DataBridgeError::Query(format!(
            "{} '{}' cannot start with a digit",
            identifier_type, identifier
        )));
    }

    // Check all characters are valid
    for c in identifier.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '$' {
            return Err(crate::DataBridgeError::Query(format!(
                "{} '{}' contains invalid character '{}'",
                identifier_type, identifier, c
            )));
        }
    }

    // Check against common SQL injection patterns
    let lower = identifier.to_lowercase();
    let dangerous_patterns = ["--", "/*", "*/", ";", "drop", "delete", "truncate"];
    for pattern in &dangerous_patterns {
        if lower.contains(pattern) {
            return Err(crate::DataBridgeError::Query(format!(
                "{} '{}' contains potentially dangerous pattern",
                identifier_type, identifier
            )));
        }
    }

    Ok(())
}

/// Validates a foreign key reference string.
///
/// Accepts two formats:
/// - "table_name" - References the "id" column in the specified table
/// - "table_name.column_name" - References a specific column
///
/// # Arguments
///
/// * `reference` - The foreign key reference string
///
/// # Returns
///
/// Ok((table_name, column_name)) if valid, Err if invalid
///
/// # Examples
///
/// ```ignore
/// let (table, col) = validate_foreign_key_reference("users")?;
/// assert_eq!(table, "users");
/// assert_eq!(col, "id");
///
/// let (table, col) = validate_foreign_key_reference("users.user_id")?;
/// assert_eq!(table, "users");
/// assert_eq!(col, "user_id");
/// ```
pub fn validate_foreign_key_reference(reference: &str) -> Result<(String, String)> {
    if reference.is_empty() {
        return Err(crate::DataBridgeError::Query(
            "Foreign key reference cannot be empty".to_string(),
        ));
    }

    // Split on '.' to check for table.column format
    let parts: Vec<&str> = reference.split('.').collect();

    match parts.len() {
        1 => {
            // Format: "table" - default to "id" column
            let table = parts[0];
            validate_identifier(table, "Foreign key table name")?;
            Ok((table.to_string(), "id".to_string()))
        }
        2 => {
            // Format: "table.column"
            let table = parts[0];
            let column = parts[1];
            validate_identifier(table, "Foreign key table name")?;
            validate_identifier(column, "Foreign key column name")?;
            Ok((table.to_string(), column.to_string()))
        }
        _ => Err(crate::DataBridgeError::Query(format!(
            "Invalid foreign key reference format '{}'. Expected 'table' or 'table.column'",
            reference
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_identifier_valid() {
        assert!(validate_identifier("users", "table").is_ok());
        assert!(validate_identifier("user_posts", "table").is_ok());
        assert!(validate_identifier("posts_v2", "table").is_ok());
        assert!(validate_identifier("_private", "column").is_ok());
        assert!(validate_identifier("col$1", "column").is_ok());
    }

    #[test]
    fn test_validate_identifier_empty() {
        let result = validate_identifier("", "table");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_identifier_starts_with_digit() {
        let result = validate_identifier("1users", "table");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot start with a digit"));
    }

    #[test]
    fn test_validate_identifier_invalid_chars() {
        assert!(validate_identifier("user-posts", "table").is_err());
        assert!(validate_identifier("user.posts", "table").is_err());
        assert!(validate_identifier("user posts", "table").is_err());
        assert!(validate_identifier("user@posts", "table").is_err());
    }

    #[test]
    fn test_validate_identifier_sql_injection() {
        assert!(validate_identifier("users--", "table").is_err());
        assert!(validate_identifier("users;DROP", "table").is_err());
        assert!(validate_identifier("users/*comment*/", "table").is_err());
    }

    #[test]
    fn test_validate_foreign_key_reference_table_only() {
        let result = validate_foreign_key_reference("users");
        assert!(result.is_ok());
        let (table, column) = result.unwrap();
        assert_eq!(table, "users");
        assert_eq!(column, "id");
    }

    #[test]
    fn test_validate_foreign_key_reference_table_and_column() {
        let result = validate_foreign_key_reference("users.user_id");
        assert!(result.is_ok());
        let (table, column) = result.unwrap();
        assert_eq!(table, "users");
        assert_eq!(column, "user_id");
    }

    #[test]
    fn test_validate_foreign_key_reference_empty() {
        let result = validate_foreign_key_reference("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot be empty"));
    }

    #[test]
    fn test_validate_foreign_key_reference_too_many_parts() {
        let result = validate_foreign_key_reference("schema.table.column");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));
    }

    #[test]
    fn test_validate_foreign_key_reference_invalid_table() {
        let result = validate_foreign_key_reference("user-posts");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_foreign_key_reference_invalid_column() {
        let result = validate_foreign_key_reference("users.user-id");
        assert!(result.is_err());
    }
}
