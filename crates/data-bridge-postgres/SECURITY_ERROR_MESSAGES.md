# Security: Sanitized Error Messages

## Overview

This document describes the error message sanitization implemented to prevent information leakage in production environments.

## Problem

Previously, error messages exposed sensitive internal schema information including:
- Table names
- Column names
- Constraint names
- Foreign key relationships
- Database structure details

### Example of Problematic Error Messages

```rust
// BEFORE - Leaks table names and constraint details
"Cannot delete from 'users': referenced by 'posts' (fk_posts_user_id)"
"Column 'email' not found"
"Table 'public.users' does not exist"
```

This information could be used by attackers to:
1. Map database schema structure
2. Identify relationships between tables
3. Craft more targeted SQL injection attacks
4. Understand business logic through table/column names

## Solution

All user-facing error messages have been sanitized to provide helpful feedback without exposing internal schema details.

### Sanitized Error Messages

```rust
// AFTER - Generic but helpful
"Cannot delete record: foreign key constraint violation. Use cascade delete or remove referencing records first."
"Column not found in result set"
"Table does not exist"
```

## Changes Made

### 1. Row Operations (`row.rs`)

#### Column Access Errors
- **Before**: `"Column '{}' not found"`
- **After**: `"Column not found in result set"`

#### CRUD Operation Errors
- **Before**: `"Insert failed: {error_details}"`
- **After**: `"Insert operation failed"`

Similar changes for:
- Batch insert → `"Batch insert operation failed"`
- Upsert → `"Upsert operation failed"`
- Find → `"Find operation failed"`
- Update → `"Update operation failed"`
- Delete → `"Delete operation failed"`
- Count → `"Count operation failed"`

#### Foreign Key Constraint Violations
- **Before**: `"Cannot delete from '{table}': referenced by '{source_table}' via column '{column}' (constraint: {constraint_name})"`
- **After**: `"Cannot delete record: foreign key constraint violation. Use cascade delete or remove referencing records first."`

#### Nested Data Processing
- **Before**: `"Failed to strip prefix '{prefix}' from key '{key}'"`
- **After**: `"Failed to process nested data structure"`

### 2. Validation (`validation.rs`)

#### Identifier Validation
- **Before**: `"{identifier_type} '{identifier}' cannot be empty"`
- **After**: `"Identifier cannot be empty"`

- **Before**: `"{identifier_type} '{identifier}' cannot start with a digit"`
- **After**: `"Identifier cannot start with a digit"`

- **Before**: `"{identifier_type} '{identifier}' contains invalid character '{char}'"`
- **After**: `"Identifier contains invalid characters. Only alphanumeric, underscore, and dollar sign allowed"`

- **Before**: `"{identifier_type} '{identifier}' contains potentially dangerous pattern"`
- **After**: `"Identifier contains potentially dangerous pattern"`

#### Foreign Key Reference Validation
- **Before**: `"Invalid foreign key reference format '{reference}'. Expected 'table' or 'table.column'"`
- **After**: `"Invalid foreign key reference format. Expected 'table' or 'table.column'"`

### 3. Schema Operations (`schema.rs`)

#### Table Existence Check
- **Before**: `"Table '{schema}.{table}' does not exist"`
- **After**: `"Table does not exist"`

## Security Benefits

1. **Prevents Schema Enumeration**: Attackers cannot discover table/column names through error messages
2. **Hides Relationships**: Foreign key relationships between tables remain hidden
3. **Protects Business Logic**: Table/column naming conventions don't reveal business logic
4. **Reduces Attack Surface**: Less information available for crafting targeted attacks

## Developer Experience

While error messages are less specific for end users, developers can still:

1. **Use Logging**: Detailed errors should be logged server-side with full context
2. **Debug Mode**: Consider a debug/development mode that shows detailed errors (NOT in production)
3. **Error Codes**: Implement error codes that map to detailed internal documentation
4. **Structured Logging**: Log the original error with full details for debugging

## Future Enhancements

Consider implementing:

1. **Error Codes**: Add unique error codes for each error type
   ```rust
   "ERR_FK_VIOLATION: Cannot delete record due to foreign key constraint"
   ```

2. **Separate Logging Field**: Store detailed error information in a separate field for server logs
   ```rust
   DataBridgeError::Validation {
       message: "Cannot delete record: foreign key constraint violation",
       details: Some("Table 'users' referenced by 'posts' via 'user_id'"),
   }
   ```

3. **Debug Mode Toggle**: Environment-based toggle for detailed errors
   ```rust
   if cfg!(debug_assertions) {
       format!("Column '{}' not found", column)
   } else {
       "Column not found in result set".to_string()
   }
   ```

## Testing

All tests have been updated and verified:
- ✅ Unit tests pass (12/12)
- ✅ Integration tests pass (29/29)
- ✅ No clippy warnings
- ✅ Error message assertions still valid (use generic phrases)

## References

- OWASP: [Information Exposure Through Error Messages](https://owasp.org/www-community/Improper_Error_Handling)
- CWE-209: [Generation of Error Message Containing Sensitive Information](https://cwe.mitre.org/data/definitions/209.html)
