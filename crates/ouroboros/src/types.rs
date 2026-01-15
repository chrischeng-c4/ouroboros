//! Core types for ouroboros
//!
//! This module provides Rust-native types exposed to Python via PyO3,
//! eliminating dependencies on external Python packages like pymongo.

use pyo3::prelude::*;
use pyo3::types::PyType;
use bson::oid::ObjectId as BsonObjectId;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// A MongoDB ObjectId implemented in Rust and exposed to Python.
///
/// This replaces the need for `bson.ObjectId` from pymongo, providing:
/// - Zero Python byte handling (all operations in Rust)
/// - Better performance for ObjectId generation and validation
/// - Seamless integration with ouroboros MongoDB operations
///
/// # Python Usage
/// ```python
/// from ouroboros import ObjectId
///
/// # Generate new ObjectId
/// oid = ObjectId.new()
///
/// # Parse from string
/// oid = ObjectId("507f1f77bcf86cd799439011")
///
/// # Validate string
/// ObjectId.is_valid("507f1f77bcf86cd799439011")  # True
/// ObjectId.is_valid("invalid")  # False
///
/// # String representation
/// str(oid)  # "507f1f77bcf86cd799439011"
/// repr(oid)  # "ObjectId('507f1f77bcf86cd799439011')"
/// ```
#[pyclass(name = "ObjectId", module = "ouroboros")]
#[derive(Clone, Debug)]
pub struct PyObjectId {
    inner: BsonObjectId,
}

impl PyObjectId {
    /// Create a new PyObjectId from a BsonObjectId
    pub fn from_bson(oid: BsonObjectId) -> Self {
        Self { inner: oid }
    }

    /// Create a new PyObjectId from a hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        BsonObjectId::parse_str(hex_str)
            .map(|oid| Self { inner: oid })
            .map_err(|_| format!(
                "'{}' is not a valid ObjectId, it must be a 24-character hex string",
                hex_str
            ))
    }

    /// Get the inner BsonObjectId
    pub fn to_bson(&self) -> BsonObjectId {
        self.inner
    }

    /// Get the hex string representation
    pub fn to_hex(&self) -> String {
        self.inner.to_hex()
    }
}

#[pymethods]
impl PyObjectId {
    /// Create a new ObjectId from a hex string.
    ///
    /// Args:
    ///     hex_str: A 24-character hexadecimal string
    ///
    /// Raises:
    ///     ValueError: If the string is not a valid ObjectId
    #[new]
    fn py_new(hex_str: &str) -> PyResult<Self> {
        BsonObjectId::parse_str(hex_str)
            .map(|oid| Self { inner: oid })
            .map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "'{}' is not a valid ObjectId, it must be a 24-character hex string",
                    hex_str
                ))
            })
    }

    /// Generate a new unique ObjectId.
    ///
    /// Returns:
    ///     A new ObjectId with a unique value
    #[classmethod]
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: BsonObjectId::new(),
        }
    }

    /// Check if a string is a valid ObjectId hex representation.
    ///
    /// Args:
    ///     value: The string to validate
    ///
    /// Returns:
    ///     True if the string is a valid 24-character hex string, False otherwise
    #[staticmethod]
    fn is_valid(value: &str) -> bool {
        BsonObjectId::parse_str(value).is_ok()
    }

    /// Parse an ObjectId from a hex string.
    ///
    /// This is an alias for the constructor, provided for API compatibility.
    ///
    /// Args:
    ///     hex_str: A 24-character hexadecimal string
    ///
    /// Returns:
    ///     An ObjectId instance
    ///
    /// Raises:
    ///     ValueError: If the string is not a valid ObjectId
    #[staticmethod]
    fn from_str(hex_str: &str) -> PyResult<Self> {
        Self::py_new(hex_str)
    }

    /// Return the hex string representation of this ObjectId.
    fn __str__(&self) -> String {
        self.inner.to_hex()
    }

    /// Return a string representation suitable for debugging.
    fn __repr__(&self) -> String {
        format!("ObjectId('{}')", self.inner.to_hex())
    }

    /// Compare this ObjectId with another for equality.
    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    /// Compare this ObjectId with another for inequality.
    fn __ne__(&self, other: &Self) -> bool {
        self.inner != other.inner
    }

    /// Return a hash value for this ObjectId.
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.inner.bytes().hash(&mut hasher);
        hasher.finish()
    }

    /// Get the binary representation of this ObjectId.
    ///
    /// Returns:
    ///     A 12-byte bytes object
    fn binary<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new(py, &self.inner.bytes())
    }

    /// Get the timestamp component of this ObjectId.
    ///
    /// Returns:
    ///     The Unix timestamp (seconds since epoch) when this ObjectId was created
    fn timestamp(&self) -> u32 {
        self.inner.timestamp().to_chrono().timestamp() as u32
    }
}

/// Register the types module with the parent module.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyObjectId>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objectid_new() {
        let oid1 = PyObjectId::from_bson(BsonObjectId::new());
        let oid2 = PyObjectId::from_bson(BsonObjectId::new());

        // Each new ObjectId should be unique
        assert_ne!(oid1.to_hex(), oid2.to_hex());

        // Should be 24 hex characters
        assert_eq!(oid1.to_hex().len(), 24);
        assert!(oid1.to_hex().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_objectid_parse_valid() {
        let hex_str = "507f1f77bcf86cd799439011";
        let oid = PyObjectId::py_new(hex_str).unwrap();
        assert_eq!(oid.to_hex(), hex_str);
    }

    #[test]
    fn test_objectid_parse_invalid_length() {
        let result = PyObjectId::py_new("123");
        assert!(result.is_err());
    }

    #[test]
    fn test_objectid_parse_invalid_chars() {
        let result = PyObjectId::py_new("507f1f77bcf86cd7994390zz");
        assert!(result.is_err());
    }

    #[test]
    fn test_objectid_is_valid() {
        assert!(PyObjectId::is_valid("507f1f77bcf86cd799439011"));
        assert!(!PyObjectId::is_valid("123"));
        assert!(!PyObjectId::is_valid("507f1f77bcf86cd7994390zz"));
        assert!(!PyObjectId::is_valid(""));
    }

    #[test]
    fn test_objectid_equality() {
        let hex_str = "507f1f77bcf86cd799439011";
        let oid1 = PyObjectId::py_new(hex_str).unwrap();
        let oid2 = PyObjectId::py_new(hex_str).unwrap();
        let oid3 = PyObjectId::py_new("507f1f77bcf86cd799439012").unwrap();

        assert!(oid1.__eq__(&oid2));
        assert!(!oid1.__eq__(&oid3));
        assert!(oid1.__ne__(&oid3));
    }

    #[test]
    fn test_objectid_hash() {
        let hex_str = "507f1f77bcf86cd799439011";
        let oid1 = PyObjectId::py_new(hex_str).unwrap();
        let oid2 = PyObjectId::py_new(hex_str).unwrap();

        // Same ObjectId should have same hash
        assert_eq!(oid1.__hash__(), oid2.__hash__());
    }

    #[test]
    fn test_objectid_str_repr() {
        let hex_str = "507f1f77bcf86cd799439011";
        let oid = PyObjectId::py_new(hex_str).unwrap();

        assert_eq!(oid.__str__(), hex_str);
        assert_eq!(oid.__repr__(), "ObjectId('507f1f77bcf86cd799439011')");
    }
}
