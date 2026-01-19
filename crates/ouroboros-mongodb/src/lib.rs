//! MongoDB implementation for ouroboros
//!
//! This crate provides a high-performance MongoDB ORM with full Beanie compatibility.
//!
//! # Features
//! - Zero-copy BSON deserialization
//! - Async/await support via tokio
//! - Query builder with Beanie-compatible API
//! - Aggregation pipeline support
//! - Index management
//! - Revision tracking
//! - State management

pub mod connection;
pub mod document;
pub mod query;
pub mod validation;

pub use connection::{Connection, PoolConfig};
pub use ouroboros_common::{DataBridgeError, Result};
pub use document::Document;
pub use validation::{
    ValidatedCollectionName, ValidatedFieldName, ObjectIdParser,
    validate_query, BsonConstraints, BsonTypeDescriptor,
    validate_field, validate_document,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
