//! MongoDB module for Python bindings
//!
//! This module provides Python bindings for MongoDB operations using PyO3.
//! All BSON serialization/deserialization happens in Rust for maximum performance.

use bson::{doc, oid::ObjectId, Bson, Document as BsonDocument, Decimal128, Binary};
use bson::spec::BinarySubtype;
use futures::TryStreamExt;
use mongodb::IndexModel;
use mongodb::options::IndexOptions;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use pyo3::conversion::IntoPyObject;
use pyo3_async_runtimes::tokio::future_into_py;
use rayon::prelude::*;
use std::sync::Arc;
use std::str::FromStr;

use data_bridge_mongodb::Connection;

// Import security modules
use crate::validation::ValidatedCollectionName;
use crate::config::{get_config, ObjectIdConversionMode, SecurityConfig};
use crate::error_handling::sanitize_mongodb_error;

// Import GIL-free conversion functions (Feature 201)
use crate::conversion::{
    extract_dict_items, items_to_bson_document,
    bson_to_serializable, serializable_to_py_dict,
    ConversionContext,
};

// Global connection instance (Week 10: Changed to RwLock for close/reset support)
use std::sync::RwLock as StdRwLock;
static CONNECTION: StdRwLock<Option<Arc<Connection>>> = StdRwLock::new(None);

/// Minimum batch size to enable parallel processing
/// Below this threshold, sequential processing is faster due to parallelization overhead
const PARALLEL_THRESHOLD: usize = 50;

/// Index information returned by list_indexes
#[derive(Debug, Clone)]
struct IndexInfo {
    key: BsonDocument,
    name: Option<String>,
    unique: Option<bool>,
    sparse: Option<bool>,
    expire_after_seconds: Option<i64>,
    background: Option<bool>,
}

/// Update result information
#[derive(Debug, Clone)]
struct UpdateResult {
    matched_count: u64,
    modified_count: u64,
    upserted_id: Option<String>,
}

/// Intermediate representation for Python values
///
/// This type allows us to extract data from Python while holding the GIL,
/// then convert to BSON without the GIL for better performance.
#[derive(Debug, Clone)]
enum ExtractedValue {
    Null,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Double(f64),
    String(String),
    ObjectIdString(String),  // Store as string, parse to ObjectId later
    DateTimeMillis(i64),     // Milliseconds since epoch
    Bytes(Vec<u8>),
    Decimal(String),         // Store as string, parse to Decimal128 later
    Array(Vec<ExtractedValue>),
    Document(Vec<(String, ExtractedValue)>),
}

/// Intermediate representation for bulk write operations
#[derive(Debug, Clone)]
enum ExtractedBulkOp {
    InsertOne {
        document: Vec<(String, ExtractedValue)>,
    },
    UpdateOne {
        filter: Vec<(String, ExtractedValue)>,
        update: Vec<(String, ExtractedValue)>,
        upsert: bool,
    },
    UpdateMany {
        filter: Vec<(String, ExtractedValue)>,
        update: Vec<(String, ExtractedValue)>,
        upsert: bool,
    },
    DeleteOne {
        filter: Vec<(String, ExtractedValue)>,
    },
    DeleteMany {
        filter: Vec<(String, ExtractedValue)>,
    },
    ReplaceOne {
        filter: Vec<(String, ExtractedValue)>,
        replacement: Vec<(String, ExtractedValue)>,
        upsert: bool,
    },
}

impl<'py> IntoPyObject<'py> for UpdateResult {
    type Target = PyDict;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);
        dict.set_item("matched_count", self.matched_count)?;
        dict.set_item("modified_count", self.modified_count)?;
        if let Some(id) = self.upserted_id {
            dict.set_item("upserted_id", id)?;
        }
        Ok(dict)
    }
}

/// Distinct result wrapper
#[derive(Debug, Clone)]
struct DistinctResult {
    values: Vec<Bson>,
}

impl<'py> IntoPyObject<'py> for DistinctResult {
    type Target = PyList;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let list = PyList::empty(py);
        for value in self.values {
            if let Ok(py_val) = bson_to_py(py, &value) {
                list.append(py_val)?;
            }
        }
        Ok(list)
    }
}

/// Bulk write result wrapper
#[derive(Debug, Clone)]
struct BulkWriteResultWrapper {
    inserted_count: i64,
    matched_count: i64,
    modified_count: i64,
    deleted_count: i64,
    upserted_count: i64,
    upserted_ids: std::collections::HashMap<i64, String>,
}

impl<'py> IntoPyObject<'py> for BulkWriteResultWrapper {
    type Target = PyDict;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);
        dict.set_item("inserted_count", self.inserted_count)?;
        dict.set_item("matched_count", self.matched_count)?;
        dict.set_item("modified_count", self.modified_count)?;
        dict.set_item("deleted_count", self.deleted_count)?;
        dict.set_item("upserted_count", self.upserted_count)?;

        // Convert upserted_ids HashMap to Python dict
        let upserted_dict = PyDict::new(py);
        for (idx, id) in self.upserted_ids {
            upserted_dict.set_item(idx, id)?;
        }
        dict.set_item("upserted_ids", upserted_dict)?;

        Ok(dict)
    }
}

impl<'py> IntoPyObject<'py> for IndexInfo {
    type Target = PyDict;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);

        // Convert keys
        let keys_dict = PyDict::new(py);
        for (k, v) in self.key.iter() {
            if let Ok(py_val) = bson_to_py(py, v) {
                keys_dict.set_item(k, py_val)?;
            }
        }
        dict.set_item("key", keys_dict)?;

        // Add optional fields
        if let Some(name) = self.name {
            dict.set_item("name", name)?;
        }
        if let Some(unique) = self.unique {
            dict.set_item("unique", unique)?;
        }
        if let Some(sparse) = self.sparse {
            dict.set_item("sparse", sparse)?;
        }
        if let Some(expire) = self.expire_after_seconds {
            dict.set_item("expireAfterSeconds", expire)?;
        }
        if let Some(bg) = self.background {
            dict.set_item("background", bg)?;
        }

        Ok(dict)
    }
}

/// Get the global connection, returning an error if not initialized
fn get_connection() -> PyResult<Arc<Connection>> {
    CONNECTION
        .read()
        .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?
        .clone()
        .ok_or_else(|| PyRuntimeError::new_err("MongoDB not initialized. Call init() first."))
}

/// Validate collection name for security
///
/// Prevents NoSQL injection via collection names
fn validate_collection_name(name: &str) -> PyResult<ValidatedCollectionName> {
    ValidatedCollectionName::new(name)
}

/// Validate MongoDB query for dangerous operators if validation is enabled
///
/// Checks the security config and validates the query if validate_queries is true.
/// Blocks dangerous operators like $where, $function, $accumulator.
///
/// # Arguments
/// * `query` - The query document to validate
///
/// # Errors
/// Returns PyValueError if dangerous operators are detected and validation is enabled
fn validate_query_if_enabled(query: &bson::Document) -> PyResult<()> {
    let config = get_config();
    if config.validate_queries {
        use crate::validation::validate_query;
        validate_query(&bson::Bson::Document(query.clone()))?;
    }
    Ok(())
}

/// Initialize MongoDB connection
///
/// Args:
///     connection_string: MongoDB connection URI (e.g., "mongodb://localhost:27017/mydb")
///
/// Returns:
///     None
///
/// Raises:
///     RuntimeError: If already initialized or connection fails
#[pyfunction]
fn init<'py>(py: Python<'py>, connection_string: String) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let conn = Connection::new(&connection_string)
            .await
            .map_err(|e| {
                use crate::error_handling::sanitize_error;
                use crate::config::get_config;
                let config = get_config();
                let error_msg = e.to_string();
                let sanitized = sanitize_error(&error_msg, !config.sanitize_errors);
                PyRuntimeError::new_err(sanitized)
            })?;

        // Check if already initialized
        {
            let read_lock = CONNECTION.read()
                .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?;
            if read_lock.is_some() {
                return Err(PyRuntimeError::new_err("MongoDB already initialized. Call close() first to reinitialize."));
            }
        }

        // Set connection
        let mut write_lock = CONNECTION.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?;
        *write_lock = Some(Arc::new(conn));

        Ok(())
    })
}

/// Get connection status
#[pyfunction]
fn is_connected() -> bool {
    CONNECTION.read()
        .ok()
        .and_then(|lock| lock.as_ref().map(|_| true))
        .unwrap_or(false)
}

/// Close the MongoDB connection (Week 10: Connection Lifecycle)
///
/// Closes and releases the current connection. After calling this,
/// init() can be called again to establish a new connection.
///
/// This is useful for:
/// - Clean shutdown
/// - Testing (reset between tests)
/// - Connection refresh/reconnection
///
/// Example:
///     >>> await init("mongodb://localhost:27017/db1")
///     >>> # ... use database ...
///     >>> await close()
///     >>> await init("mongodb://localhost:27017/db2")  # Different database
#[pyfunction]
fn close<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let mut write_lock = CONNECTION.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?;

        if write_lock.is_none() {
            return Err(PyRuntimeError::new_err("No active connection to close"));
        }

        // Drop the connection (Arc will be dropped when no references remain)
        *write_lock = None;

        Ok(())
    })
}

/// Reset the connection (Week 10: Connection Lifecycle)
///
/// Clear the connection without async operation. Useful for testing.
///
/// Note: This is a synchronous operation for convenience in tests.
/// For production use, prefer close() which is async.
///
/// Example:
///     >>> reset()  # Synchronous, for testing
///     >>> await init("mongodb://localhost:27017/test")
#[pyfunction]
fn reset() -> PyResult<()> {
    let mut write_lock = CONNECTION.write()
        .map_err(|e| PyRuntimeError::new_err(format!("Connection lock poisoned: {}", e)))?;
    *write_lock = None;
    Ok(())
}

/// Available features in this build
#[pyfunction]
fn available_features() -> Vec<String> {
    vec!["mongodb".to_string()]
}

/// Extract Python value to intermediate representation (call with GIL held)
fn extract_py_value(py: Python<'_>, value: &Bound<'_, PyAny>, config: &SecurityConfig) -> PyResult<ExtractedValue> {
    // None
    if value.is_none() {
        return Ok(ExtractedValue::Null);
    }

    // Boolean (must check before int since bool is subclass of int in Python)
    if value.is_instance_of::<pyo3::types::PyBool>() {
        return Ok(ExtractedValue::Bool(value.extract::<bool>()?));
    }

    // Bytes
    if let Ok(bytes) = value.downcast::<PyBytes>() {
        return Ok(ExtractedValue::Bytes(bytes.as_bytes().to_vec()));
    }

    // Get type name for checking special types
    let type_name = value.get_type().name().map(|s| s.to_string()).unwrap_or_default();

    // DateTime
    if type_name == "datetime" {
        if let Ok(timestamp_method) = value.getattr("timestamp") {
            if let Ok(timestamp_result) = timestamp_method.call0() {
                if let Ok(timestamp) = timestamp_result.extract::<f64>() {
                    let millis = (timestamp * 1000.0) as i64;
                    return Ok(ExtractedValue::DateTimeMillis(millis));
                }
            }
        }
        return Err(PyValueError::new_err("Failed to convert datetime to timestamp"));
    }

    // Date
    if type_name == "date" {
        // Convert date to datetime at midnight UTC
        if let Ok(datetime_module) = py.import("datetime") {
            if let Ok(datetime_class) = datetime_module.getattr("datetime") {
                if let Ok(year) = value.getattr("year") {
                    if let Ok(month) = value.getattr("month") {
                    if let Ok(day) = value.getattr("day") {
                        if let Ok(dt) = datetime_class.call1((year, month, day)) {
                            if let Ok(timestamp_method) = dt.getattr("timestamp") {
                                if let Ok(ts_result) = timestamp_method.call0() {
                                    if let Ok(timestamp) = ts_result.extract::<f64>() {
                                        let millis = (timestamp * 1000.0) as i64;
                                        return Ok(ExtractedValue::DateTimeMillis(millis));
                                    }
                                }
                            }
                        }
                    }
                    }
                }
            }
        }
        return Err(PyValueError::new_err("Failed to convert date to datetime"));
    }

    // Decimal
    if type_name == "Decimal" {
        if let Ok(s) = value.str() {
            return Ok(ExtractedValue::Decimal(s.to_string()));
        }
        return Err(PyValueError::new_err("Failed to convert Decimal to string"));
    }

    // ObjectId wrapper
    if type_name == "PydanticObjectId" || type_name == "ObjectId" {
        if let Ok(s) = value.str() {
            return Ok(ExtractedValue::ObjectIdString(s.to_string()));
        }
        return Err(PyValueError::new_err("Failed to convert ObjectId to string"));
    }

    // Integer (try i32 first, then i64)
    if let Ok(i) = value.extract::<i32>() {
        return Ok(ExtractedValue::Int32(i));
    }
    if let Ok(i) = value.extract::<i64>() {
        return Ok(ExtractedValue::Int64(i));
    }

    // Float
    if let Ok(f) = value.extract::<f64>() {
        return Ok(ExtractedValue::Double(f));
    }

    // String
    if let Ok(s) = value.extract::<String>() {
        // Check if should convert to ObjectId
        let should_convert = match config.objectid_mode {
            ObjectIdConversionMode::Lenient => {
                s.len() == 24 && s.chars().all(|c| c.is_ascii_hexdigit())
            }
            ObjectIdConversionMode::TypeHinted | ObjectIdConversionMode::Strict => false,
        };

        if should_convert {
            return Ok(ExtractedValue::ObjectIdString(s));
        }
        return Ok(ExtractedValue::String(s));
    }

    // Dict
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut doc = Vec::with_capacity(dict.len());
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            let extracted = extract_py_value(py, &value, config)?;
            doc.push((key, extracted));
        }
        return Ok(ExtractedValue::Document(doc));
    }

    // List
    if let Ok(list) = value.downcast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(extract_py_value(py, &item, config)?);
        }
        return Ok(ExtractedValue::Array(arr));
    }

    // Fallback: try to convert to string
    if let Ok(s) = value.str() {
        return Ok(ExtractedValue::String(s.to_string()));
    }

    Err(PyValueError::new_err(format!(
        "Unsupported type for BSON conversion: {:?}",
        type_name
    )))
}

/// Convert extracted value to BSON (can be called without GIL)
fn extracted_to_bson(value: ExtractedValue) -> Bson {
    match value {
        ExtractedValue::Null => Bson::Null,
        ExtractedValue::Bool(b) => Bson::Boolean(b),
        ExtractedValue::Int32(i) => Bson::Int32(i),
        ExtractedValue::Int64(i) => Bson::Int64(i),
        ExtractedValue::Double(f) => Bson::Double(f),
        ExtractedValue::String(s) => Bson::String(s),
        ExtractedValue::ObjectIdString(s) => {
            ObjectId::parse_str(&s)
                .map(Bson::ObjectId)
                .unwrap_or_else(|_| Bson::String(s))
        }
        ExtractedValue::DateTimeMillis(millis) => {
            Bson::DateTime(bson::DateTime::from_millis(millis))
        }
        ExtractedValue::Bytes(b) => Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: b,
        }),
        ExtractedValue::Decimal(s) => {
            Decimal128::from_str(&s)
                .map(Bson::Decimal128)
                .unwrap_or_else(|_| Bson::String(s))
        }
        ExtractedValue::Array(arr) => {
            Bson::Array(arr.into_iter().map(extracted_to_bson).collect())
        }
        ExtractedValue::Document(doc) => {
            let mut bson_doc = BsonDocument::new();
            for (key, value) in doc {
                bson_doc.insert(key, extracted_to_bson(value));
            }
            Bson::Document(bson_doc)
        }
    }
}

/// Convert BSON to extracted value (can be called without GIL)
fn bson_to_extracted(value: &Bson) -> ExtractedValue {
    match value {
        Bson::Null => ExtractedValue::Null,
        Bson::Boolean(b) => ExtractedValue::Bool(*b),
        Bson::Int32(i) => ExtractedValue::Int32(*i),
        Bson::Int64(i) => ExtractedValue::Int64(*i),
        Bson::Double(f) => ExtractedValue::Double(*f),
        Bson::String(s) => ExtractedValue::String(s.clone()),
        Bson::ObjectId(oid) => ExtractedValue::ObjectIdString(oid.to_hex()),
        Bson::DateTime(dt) => ExtractedValue::DateTimeMillis(dt.timestamp_millis()),
        Bson::Binary(bin) => ExtractedValue::Bytes(bin.bytes.clone()),
        Bson::Decimal128(dec) => ExtractedValue::Decimal(dec.to_string()),
        Bson::Array(arr) => {
            ExtractedValue::Array(arr.iter().map(bson_to_extracted).collect())
        }
        Bson::Document(doc) => {
            let fields: Vec<(String, ExtractedValue)> = doc
                .iter()
                .map(|(k, v)| (k.clone(), bson_to_extracted(v)))
                .collect();
            ExtractedValue::Document(fields)
        }
        // Handle other BSON types by converting to string
        _ => ExtractedValue::String(format!("{:?}", value)),
    }
}

/// Convert extracted value to Python (must be called with GIL held)
fn extracted_to_py(py: Python<'_>, value: ExtractedValue) -> PyResult<PyObject> {
    match value {
        ExtractedValue::Null => Ok(py.None()),
        ExtractedValue::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        ExtractedValue::Int32(i) => Ok(i.into_pyobject(py)?.to_owned().into_any().unbind()),
        ExtractedValue::Int64(i) => Ok(i.into_pyobject(py)?.to_owned().into_any().unbind()),
        ExtractedValue::Double(f) => Ok(f.into_pyobject(py)?.to_owned().into_any().unbind()),
        ExtractedValue::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        ExtractedValue::ObjectIdString(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        ExtractedValue::DateTimeMillis(millis) => {
            // Convert to Python datetime
            let datetime_module = py.import("datetime")?;
            let datetime_class = datetime_module.getattr("datetime")?;
            let from_timestamp = datetime_class.getattr("fromtimestamp")?;
            let timestamp_secs = (millis as f64) / 1000.0;
            Ok(from_timestamp.call1((timestamp_secs,))?.into())
        }
        ExtractedValue::Bytes(b) => Ok(PyBytes::new(py, &b).into()),
        ExtractedValue::Decimal(s) => {
            // Return as string, let Python convert if needed
            Ok(s.into_pyobject(py)?.into_any().unbind())
        }
        ExtractedValue::Array(arr) => {
            let py_list = PyList::empty(py);
            for item in arr {
                py_list.append(extracted_to_py(py, item)?)?;
            }
            Ok(py_list.into())
        }
        ExtractedValue::Document(doc) => {
            let py_dict = PyDict::new(py);
            for (key, value) in doc {
                py_dict.set_item(key, extracted_to_py(py, value)?)?;
            }
            Ok(py_dict.into())
        }
    }
}

/// Extract dict fields to intermediate representation
fn extract_dict_fields(py: Python<'_>, dict: &Bound<'_, PyDict>, config: &SecurityConfig) -> PyResult<Vec<(String, ExtractedValue)>> {
    let mut fields = Vec::with_capacity(dict.len());
    for (key, value) in dict.iter() {
        let key: String = key.extract()?;
        let extracted = extract_py_value(py, &value, config)?;
        fields.push((key, extracted));
    }
    Ok(fields)
}

/// Convert ExtractedBulkOp to tuple format used by bulk_write
fn extracted_bulk_op_to_tuple(op: ExtractedBulkOp) -> (String, BsonDocument, Option<BsonDocument>, bool) {
    match op {
        ExtractedBulkOp::InsertOne { document } => {
            let mut doc = BsonDocument::new();
            for (key, value) in document {
                doc.insert(key, extracted_to_bson(value));
            }
            ("insert_one".to_string(), doc, None, false)
        }
        ExtractedBulkOp::UpdateOne { filter, update, upsert } => {
            let mut filter_doc = BsonDocument::new();
            for (key, value) in filter {
                filter_doc.insert(key, extracted_to_bson(value));
            }
            let mut update_doc = BsonDocument::new();
            for (key, value) in update {
                update_doc.insert(key, extracted_to_bson(value));
            }
            ("update_one".to_string(), filter_doc, Some(update_doc), upsert)
        }
        ExtractedBulkOp::UpdateMany { filter, update, upsert } => {
            let mut filter_doc = BsonDocument::new();
            for (key, value) in filter {
                filter_doc.insert(key, extracted_to_bson(value));
            }
            let mut update_doc = BsonDocument::new();
            for (key, value) in update {
                update_doc.insert(key, extracted_to_bson(value));
            }
            ("update_many".to_string(), filter_doc, Some(update_doc), upsert)
        }
        ExtractedBulkOp::DeleteOne { filter } => {
            let mut filter_doc = BsonDocument::new();
            for (key, value) in filter {
                filter_doc.insert(key, extracted_to_bson(value));
            }
            ("delete_one".to_string(), filter_doc, None, false)
        }
        ExtractedBulkOp::DeleteMany { filter } => {
            let mut filter_doc = BsonDocument::new();
            for (key, value) in filter {
                filter_doc.insert(key, extracted_to_bson(value));
            }
            ("delete_many".to_string(), filter_doc, None, false)
        }
        ExtractedBulkOp::ReplaceOne { filter, replacement, upsert } => {
            let mut filter_doc = BsonDocument::new();
            for (key, value) in filter {
                filter_doc.insert(key, extracted_to_bson(value));
            }
            let mut replacement_doc = BsonDocument::new();
            for (key, value) in replacement {
                replacement_doc.insert(key, extracted_to_bson(value));
            }
            ("replace_one".to_string(), filter_doc, Some(replacement_doc), upsert)
        }
    }
}

/// Convert Python dict to BSON document
fn py_dict_to_bson(py: Python<'_>, dict: &Bound<'_, PyDict>) -> PyResult<BsonDocument> {
    // Get config once per document instead of per-field
    let config = get_config();
    let mut doc = BsonDocument::new();

    for (key, value) in dict.iter() {
        let key: String = key.extract()?;
        let bson_value = py_to_bson(py, &value, &config)?;
        doc.insert(key, bson_value);
    }

    Ok(doc)
}

/// Convert Python value to BSON value
///
/// # Arguments
/// * `py` - Python interpreter handle
/// * `value` - Python value to convert
/// * `config` - Security configuration (passed to avoid repeated RwLock acquisition)
fn py_to_bson(py: Python<'_>, value: &Bound<'_, PyAny>, config: &SecurityConfig) -> PyResult<Bson> {
    // None
    if value.is_none() {
        return Ok(Bson::Null);
    }

    // Boolean (must check before int since bool is subclass of int in Python)
    if let Ok(b) = value.extract::<bool>() {
        // Check if it's actually a bool, not just truthy int
        if value.is_instance_of::<pyo3::types::PyBool>() {
            return Ok(Bson::Boolean(b));
        }
    }

    // Bytes -> Binary
    if let Ok(bytes) = value.downcast::<PyBytes>() {
        let data = bytes.as_bytes().to_vec();
        return Ok(Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: data,
        }));
    }

    // Get type name for checking special types
    let type_name = value.get_type().name().map(|s| s.to_string()).unwrap_or_default();

    // DateTime (check by type name since PyDateTime not available in abi3)
    if type_name == "datetime" {
        // Get timestamp from datetime
        if let Ok(timestamp) = value.call_method0("timestamp") {
            if let Ok(ts) = timestamp.extract::<f64>() {
                let millis = (ts * 1000.0) as i64;
                return Ok(Bson::DateTime(bson::DateTime::from_millis(millis)));
            }
        }
    }

    // Check for date (without time)
    if type_name == "date" {
        // Convert date to datetime at midnight UTC
        let datetime_mod = py.import("datetime")?;
        let datetime_cls = datetime_mod.getattr("datetime")?;
        let combined = datetime_cls.call_method1("combine", (value, datetime_mod.getattr("time")?.call0()?))?;
        if let Ok(timestamp) = combined.call_method0("timestamp") {
            if let Ok(ts) = timestamp.extract::<f64>() {
                let millis = (ts * 1000.0) as i64;
                return Ok(Bson::DateTime(bson::DateTime::from_millis(millis)));
            }
        }
    }
    // Check for Decimal type (Python's decimal.Decimal)
    if type_name == "Decimal" {
        if let Ok(s) = value.str() {
            let decimal_str = s.to_string();
            if let Ok(dec) = Decimal128::from_str(&decimal_str) {
                return Ok(Bson::Decimal128(dec));
            }
        }
    }

    // Check for ObjectId wrapper class
    if type_name == "ObjectId" || type_name == "PydanticObjectId" {
        if let Ok(s) = value.str() {
            let oid_str = s.to_string();
            if let Ok(oid) = ObjectId::parse_str(&oid_str) {
                return Ok(Bson::ObjectId(oid));
            }
        }
    }

    // Integer
    if let Ok(i) = value.extract::<i64>() {
        return Ok(Bson::Int64(i));
    }

    // Float
    if let Ok(f) = value.extract::<f64>() {
        return Ok(Bson::Double(f));
    }

    // String
    if let Ok(s) = value.extract::<String>() {
        // Security: Use type-aware ObjectId conversion to prevent NoSQL injection
        // Note: config is passed as parameter to avoid repeated RwLock acquisition

        let should_convert = match config.objectid_mode {
            ObjectIdConversionMode::Lenient => {
                // Backward compatible: auto-convert 24-char hex strings
                // DEPRECATED: Will be removed in v2.0
                s.len() == 24 && s.chars().all(|c| c.is_ascii_hexdigit())
            }
            ObjectIdConversionMode::TypeHinted => {
                // Secure: Only convert if type_name indicates ObjectId
                // This relies on explicit wrapper classes (already handled above at lines 274-282)
                // Plain strings with 24 hex chars will NOT be auto-converted
                false
            }
            ObjectIdConversionMode::Strict => {
                // Most secure: Never auto-convert, require explicit wrapper
                false
            }
        };

        if should_convert {
            if let Ok(oid) = ObjectId::parse_str(&s) {
                // Emit deprecation warning for Lenient mode
                if config.objectid_mode == ObjectIdConversionMode::Lenient {
                    eprintln!(
                        "WARNING: Auto-converting string '{}...' to ObjectId. \
                        This behavior is deprecated and will be removed in v2.0. \
                        Use explicit ObjectId wrapper or set ObjectIdConversionMode.TYPE_HINTED.",
                        &s[..8]
                    );
                }
                return Ok(Bson::ObjectId(oid));
            }
        }
        return Ok(Bson::String(s));
    }

    // Dict
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut doc = BsonDocument::new();
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            let bson_value = py_to_bson(py, &value, config)?;
            doc.insert(key, bson_value);
        }
        return Ok(Bson::Document(doc));
    }

    // List
    if let Ok(list) = value.downcast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(py_to_bson(py, &item, config)?);
        }
        return Ok(Bson::Array(arr));
    }

    // Fallback: try to convert to string
    if let Ok(s) = value.str() {
        return Ok(Bson::String(s.to_string()));
    }

    Err(PyValueError::new_err(format!(
        "Cannot convert Python type to BSON: {:?}",
        value.get_type().name()
    )))
}

/// Convert BSON value to Python value
fn bson_to_py(py: Python<'_>, bson: &Bson) -> PyResult<PyObject> {
    match bson {
        Bson::Null => Ok(py.None()),
        Bson::Boolean(b) => {
            let obj = (*b).into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::Int32(i) => {
            let obj = (*i).into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::Int64(i) => {
            let obj = (*i).into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::Double(f) => {
            let obj = (*f).into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::String(s) => {
            let obj = s.as_str().into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::ObjectId(oid) => {
            let obj = oid.to_hex().into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
        Bson::Document(doc) => {
            let dict = PyDict::new(py);
            for (key, value) in doc.iter() {
                dict.set_item(key, bson_to_py(py, value)?)?;
            }
            Ok(dict.into())
        }
        Bson::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(bson_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        Bson::DateTime(dt) => {
            // Convert to Python datetime
            let datetime_mod = py.import("datetime")?;
            let datetime_cls = datetime_mod.getattr("datetime")?;
            // Get milliseconds and convert to datetime
            let millis = dt.timestamp_millis();
            let secs = millis / 1000;
            let micros = ((millis % 1000) * 1000) as u32;
            let utc = datetime_mod.getattr("timezone")?.getattr("utc")?;
            let dt_obj = datetime_cls.call_method1("fromtimestamp", (secs as f64 + micros as f64 / 1_000_000.0, utc))?;
            Ok(dt_obj.into())
        }
        Bson::Binary(binary) => {
            // Convert to Python bytes
            Ok(PyBytes::new(py, &binary.bytes).into())
        }
        Bson::Decimal128(dec) => {
            // Convert to Python Decimal
            let decimal_mod = py.import("decimal")?;
            let decimal_cls = decimal_mod.getattr("Decimal")?;
            let dec_str = dec.to_string();
            let dec_obj = decimal_cls.call1((dec_str,))?;
            Ok(dec_obj.into())
        }
        Bson::RegularExpression(regex) => {
            // Return as dict with pattern and options
            let dict = PyDict::new(py);
            dict.set_item("$regex", &regex.pattern)?;
            dict.set_item("$options", &regex.options)?;
            Ok(dict.into())
        }
        Bson::Timestamp(ts) => {
            // Return as dict with time and increment
            let dict = PyDict::new(py);
            dict.set_item("t", ts.time)?;
            dict.set_item("i", ts.increment)?;
            Ok(dict.into())
        }
        _ => {
            // For other types, convert to string representation
            let obj = bson.to_string().into_pyobject(py).unwrap();
            Ok(obj.to_owned().into_any().unbind())
        }
    }
}

/// Convert BSON document to Python dict
fn bson_doc_to_py_dict(py: Python<'_>, doc: &BsonDocument) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    for (key, value) in doc.iter() {
        dict.set_item(key, bson_to_py(py, value)?)?;
    }
    Ok(dict.into())
}

/// MongoDB Document class for Python
///
/// This class provides a Python interface to MongoDB documents with full CRUD support.
/// All BSON serialization happens in Rust for maximum performance.
///
/// Example:
///     >>> from data_bridge.mongodb import Document, init
///     >>> await init("mongodb://localhost:27017/mydb")
///     >>> doc = Document("users", {"name": "Alice", "age": 30})
///     >>> await doc.save()
///     >>> found = await Document.find_one("users", {"name": "Alice"})
#[pyclass(name = "Document")]
#[derive(Clone)]
pub struct RustDocument {
    /// Collection name
    collection_name: String,
    /// Document data as BSON
    data: BsonDocument,
    /// Document ObjectId (if saved)
    id: Option<ObjectId>,
}

#[pymethods]
impl RustDocument {
    /// Create a new document
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     data: Document data as a dict (optional)
    ///
    /// Returns:
    ///     A new Document instance
    #[new]
    #[pyo3(signature = (collection_name, data=None))]
    fn new(py: Python<'_>, collection_name: String, data: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let bson_data = match data {
            Some(dict) => py_dict_to_bson(py, dict)?,
            None => BsonDocument::new(),
        };

        // Extract _id if present
        let id = bson_data
            .get("_id")
            .and_then(|v| v.as_object_id());

        Ok(Self {
            collection_name,
            data: bson_data,
            id,
        })
    }

    /// Get the document's ObjectId as a hex string
    #[getter]
    fn id(&self) -> Option<String> {
        self.id.map(|oid| oid.to_hex())
    }

    /// Get the collection name
    #[getter]
    fn collection(&self) -> &str {
        &self.collection_name
    }

    /// Get document data as a Python dict
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        bson_doc_to_py_dict(py, &self.data)
    }

    /// Set a field value
    fn set(&mut self, py: Python<'_>, key: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        let config = get_config();
        let bson_value = py_to_bson(py, &value, &config)?;
        self.data.insert(key, bson_value);
        Ok(())
    }

    /// Get a field value
    fn get(&self, py: Python<'_>, key: &str) -> PyResult<Option<PyObject>> {
        match self.data.get(key) {
            Some(value) => Ok(Some(bson_to_py(py, value)?)),
            None => Ok(None),
        }
    }

    /// Save the document to MongoDB (insert or update)
    ///
    /// Returns:
    ///     The document's ObjectId as a hex string
    fn save<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&self.collection_name)?.into_string();

        let conn = get_connection()?;
        let mut data = self.data.clone();
        let existing_id = self.id;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            if let Some(id) = existing_id {
                // Update existing document
                data.remove("_id");
                collection
                    .update_one(doc! { "_id": id }, doc! { "$set": data })
                    .await
                    .map_err(sanitize_mongodb_error)?;
                Ok(id.to_hex())
            } else {
                // Insert new document
                let result = collection
                    .insert_one(data)
                    .await
                    .map_err(sanitize_mongodb_error)?;

                let id = result
                    .inserted_id
                    .as_object_id()
                    .ok_or_else(|| PyRuntimeError::new_err("Invalid inserted ID"))?;

                Ok(id.to_hex())
            }
        })
    }

    /// Save the document to MongoDB with type validation
    ///
    /// Args:
    ///     schema: Type schema from Python (dict of field_name -> type_descriptor)
    ///
    /// Returns:
    ///     The document's ObjectId as a hex string
    ///
    /// Validates the document against the provided schema before saving.
    fn save_validated<'py>(
        &mut self,
        py: Python<'py>,
        schema: &Bound<'_, pyo3::types::PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use crate::validation::{BsonTypeDescriptor, validate_document};

        // Security: Validate collection name
        let validated_name = validate_collection_name(&self.collection_name)?.into_string();

        // Convert Python schema to Rust type descriptors
        let mut rust_schema = std::collections::HashMap::new();
        for (key, value) in schema.iter() {
            let field_name: String = key.extract()?;
            let type_dict = value.downcast::<pyo3::types::PyDict>()?;
            let type_descriptor = BsonTypeDescriptor::from_py_dict(py, type_dict)?;
            rust_schema.insert(field_name, type_descriptor);
        }

        // Validate document against schema
        validate_document(&self.data, &rust_schema)?;

        // If validation passes, proceed with normal save
        let conn = get_connection()?;
        let mut data = self.data.clone();
        let existing_id = self.id;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            if let Some(id) = existing_id {
                // Update existing document
                data.remove("_id");
                collection
                    .update_one(doc! { "_id": id }, doc! { "$set": data })
                    .await
                    .map_err(sanitize_mongodb_error)?;
                Ok(id.to_hex())
            } else {
                // Insert new document
                let result = collection
                    .insert_one(data)
                    .await
                    .map_err(sanitize_mongodb_error)?;

                let id = result
                    .inserted_id
                    .as_object_id()
                    .ok_or_else(|| PyRuntimeError::new_err("Invalid inserted ID"))?;

                Ok(id.to_hex())
            }
        })
    }

    /// Delete this document from MongoDB
    ///
    /// Returns:
    ///     True if document was deleted, False if not found
    fn delete<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&self.collection_name)?.into_string();

        let conn = get_connection()?;
        let id = self
            .id
            .ok_or_else(|| PyRuntimeError::new_err("Document has no _id"))?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .delete_one(doc! { "_id": id })
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(result.deleted_count > 0)
        })
    }

    /// Find a single document matching the filter
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///
    /// Returns:
    ///     A Document instance or None if not found
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter=None))]
    fn find_one<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // T041: Phase 1 - Extract Python data (GIL held, minimal work)
        let config = get_config();
        let context = ConversionContext {
            security_config: config,
            max_depth: 100,        // MongoDB default nesting limit
            max_size: 16 * 1024 * 1024,  // 16MB MongoDB document limit
            strict_types: true,
        };

        let filter_items = match filter {
            Some(dict) => extract_dict_items(py, dict, &context)?,
            None => vec![],
        };

        future_into_py(py, async move {
            // T042: Phase 2 - Convert to BSON (pure Rust, no GIL needed)
            let filter_doc = if filter_items.is_empty() {
                doc! {}
            } else {
                items_to_bson_document(&filter_items)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
            };

            // Security: Validate query for dangerous operators
            validate_query_if_enabled(&filter_doc)?;

            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .find_one(filter_doc)
                .await
                .map_err(sanitize_mongodb_error)?;

            // T043: Convert BSON result to PyDict
            match result {
                Some(doc) => {
                    // Pure Rust conversion (no GIL needed)
                    let serializable = bson_to_serializable(&Bson::Document(doc));

                    // Only acquire GIL for creating Python objects
                    Python::with_gil(|py| {
                        let py_dict = serializable_to_py_dict(py, &serializable)?;
                        Ok(Some(py_dict.unbind()))
                    })
                }
                None => Ok(None),
            }
        })
    }

    /// Find all documents matching the filter
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict (optional)
    ///
    /// Returns:
    ///     A list of Document instances
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter=None))]
    fn find<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = match filter {
            Some(dict) => py_dict_to_bson(py, dict)?,
            None => doc! {},
        };

        // Security: Validate query for dangerous operators
        validate_query_if_enabled(&filter_doc)?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let cursor = collection
                .find(filter_doc)
                .await
                .map_err(sanitize_mongodb_error)?;

            let docs: Vec<BsonDocument> = cursor
                .try_collect()
                .await
                .map_err(sanitize_mongodb_error)?;

            let results: Vec<RustDocument> = docs
                .into_iter()
                .map(|doc| {
                    let id = doc.get("_id").and_then(|v| v.as_object_id());
                    RustDocument {
                        collection_name: collection_name.clone(),
                        data: doc,
                        id,
                    }
                })
                .collect();

            Ok(results)
        })
    }

    /// Find a document by its ObjectId
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     id: ObjectId as a hex string
    ///
    /// Returns:
    ///     A Document instance or None if not found
    #[staticmethod]
    fn find_by_id<'py>(
        py: Python<'py>,
        collection_name: String,
        id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let oid = ObjectId::parse_str(&id)
            .map_err(|e| PyValueError::new_err(format!("Invalid ObjectId: {}", e)))?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .find_one(doc! { "_id": oid })
                .await
                .map_err(sanitize_mongodb_error)?;

            match result {
                Some(doc) => Ok(Some(RustDocument {
                    collection_name,
                    data: doc,
                    id: Some(oid),
                })),
                None => Ok(None),
            }
        })
    }

    /// Update documents matching the filter
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///     update: Update document as a dict (will be wrapped in $set)
    ///
    /// Returns:
    ///     Number of documents modified
    #[staticmethod]
    fn update_one<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
        update: &Bound<'_, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Phase 1: Extract Python data (GIL held, minimal work)
        let config = get_config();
        let context = ConversionContext {
            security_config: config,
            max_depth: 100,
            max_size: 16 * 1024 * 1024,
            strict_types: true,
        };

        let filter_items = extract_dict_items(py, filter, &context)?;
        let update_items = extract_dict_items(py, update, &context)?;

        future_into_py(py, async move {
            // Phase 2: Convert to BSON (pure Rust, no GIL)
            let filter_doc = items_to_bson_document(&filter_items)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            validate_query_if_enabled(&filter_doc)?;

            let update_doc = items_to_bson_document(&update_items)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .update_one(filter_doc, doc! { "$set": update_doc })
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(result.modified_count)
        })
    }

    /// Delete documents matching the filter
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///
    /// Returns:
    ///     Number of documents deleted
    #[staticmethod]
    fn delete_many<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Phase 1: Extract Python data (GIL held, minimal work)
        let config = get_config();
        let context = ConversionContext {
            security_config: config,
            max_depth: 100,
            max_size: 16 * 1024 * 1024,
            strict_types: true,
        };

        let filter_items = extract_dict_items(py, filter, &context)?;

        future_into_py(py, async move {
            // Phase 2: Convert to BSON (pure Rust, no GIL)
            let filter_doc = items_to_bson_document(&filter_items)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            validate_query_if_enabled(&filter_doc)?;

            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .delete_many(filter_doc)
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(result.deleted_count)
        })
    }

    /// Count documents matching the filter
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict (optional)
    ///
    /// Returns:
    ///     Number of matching documents
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter=None))]
    fn count<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Phase 1: Extract Python data (GIL held, minimal work)
        let config = get_config();
        let context = ConversionContext {
            security_config: config,
            max_depth: 100,
            max_size: 16 * 1024 * 1024,
            strict_types: true,
        };

        let filter_items = match filter {
            Some(dict) => extract_dict_items(py, dict, &context)?,
            None => vec![],
        };

        future_into_py(py, async move {
            // Phase 2: Convert to BSON (pure Rust, no GIL)
            let filter_doc = if filter_items.is_empty() {
                doc! {}
            } else {
                items_to_bson_document(&filter_items)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
            };

            validate_query_if_enabled(&filter_doc)?;

            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let count = collection
                .count_documents(filter_doc)
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(count)
        })
    }

    /// Find documents with sorting, pagination, and projection options
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict (optional)
    ///     sort: Sort specification as a dict (optional)
    ///     skip: Number of documents to skip (optional)
    ///     limit: Maximum documents to return (optional)
    ///     projection: Fields to include/exclude (optional)
    ///
    /// Returns:
    ///     A list of Document instances
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter=None, sort=None, skip=None, limit=None, projection=None))]
    fn find_with_options<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: Option<&Bound<'_, PyDict>>,
        sort: Option<&Bound<'_, PyDict>>,
        skip: Option<u64>,
        limit: Option<i64>,
        projection: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = match filter {
            Some(dict) => py_dict_to_bson(py, dict)?,
            None => doc! {},
        };
        let sort_doc = match sort {
            Some(dict) => Some(py_dict_to_bson(py, dict)?),
            None => None,
        };
        let projection_doc = match projection {
            Some(dict) => Some(py_dict_to_bson(py, dict)?),
            None => None,
        };

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            // Build find options
            let mut find_options = mongodb::options::FindOptions::default();
            if let Some(sort) = sort_doc {
                find_options.sort = Some(sort);
            }
            if let Some(skip_val) = skip {
                find_options.skip = Some(skip_val);
            }
            if let Some(limit_val) = limit {
                find_options.limit = Some(limit_val);
            }
            if let Some(proj) = projection_doc {
                find_options.projection = Some(proj);
            }

            let cursor = collection
                .find(filter_doc)
                .with_options(find_options)
                .await
                .map_err(sanitize_mongodb_error)?;

            let docs: Vec<BsonDocument> = cursor
                .try_collect()
                .await
                .map_err(sanitize_mongodb_error)?;

            let results: Vec<RustDocument> = docs
                .into_iter()
                .map(|doc| {
                    let id = doc.get("_id").and_then(|v| v.as_object_id());
                    RustDocument {
                        collection_name: collection_name.clone(),
                        data: doc,
                        id,
                    }
                })
                .collect();

            Ok(results)
        })
    }

    /// Insert multiple documents
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     documents: List of document dicts to insert
    ///
    /// Returns:
    ///     List of inserted ObjectIds as hex strings
    #[staticmethod]
    fn insert_many<'py>(
        py: Python<'py>,
        collection_name: String,
        documents: &Bound<'_, PyList>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Phase 1: Extract Python data (GIL held, minimal work)
        let config = get_config();
        let extracted: Vec<Vec<(String, ExtractedValue)>> = {
            let mut result = Vec::with_capacity(documents.len());
            for item in documents.iter() {
                if let Ok(dict) = item.downcast::<PyDict>() {
                    let mut doc = Vec::with_capacity(dict.len());
                    for (key, value) in dict.iter() {
                        let key: String = key.extract()?;
                        let extracted_val = extract_py_value(py, &value, &config)?;
                        doc.push((key, extracted_val));
                    }
                    result.push(doc);
                } else {
                    return Err(PyValueError::new_err("All items must be dicts"));
                }
            }
            result
        };

        // Phase 2: Convert to BSON (GIL released!)
        let bson_docs: Vec<BsonDocument> = py.allow_threads(|| {
            if extracted.len() >= PARALLEL_THRESHOLD {
                // Parallel conversion for large batches
                extracted
                    .into_par_iter()
                    .map(|doc| {
                        let mut bson_doc = BsonDocument::new();
                        for (key, value) in doc {
                            bson_doc.insert(key, extracted_to_bson(value));
                        }
                        bson_doc
                    })
                    .collect()
            } else {
                // Sequential for small batches
                extracted
                    .into_iter()
                    .map(|doc| {
                        let mut bson_doc = BsonDocument::new();
                        for (key, value) in doc {
                            bson_doc.insert(key, extracted_to_bson(value));
                        }
                        bson_doc
                    })
                    .collect()
            }
        });

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .insert_many(bson_docs)
                .await
                .map_err(sanitize_mongodb_error)?;

            let ids: Vec<String> = result
                .inserted_ids
                .values()
                .filter_map(|v| v.as_object_id().map(|oid| oid.to_hex()))
                .collect();

            Ok(ids)
        })
    }

    /// Update multiple documents matching the filter
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///     update: Update document as a dict (should include $set, $inc, etc.)
    ///
    /// Returns:
    ///     Number of documents modified
    #[staticmethod]
    fn update_many<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
        update: &Bound<'_, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Phase 1: Extract Python data (GIL held, minimal work)
        let config = get_config();
        let context = ConversionContext {
            security_config: config,
            max_depth: 100,
            max_size: 16 * 1024 * 1024,
            strict_types: true,
        };

        let filter_items = extract_dict_items(py, filter, &context)?;
        let update_items = extract_dict_items(py, update, &context)?;

        future_into_py(py, async move {
            // Phase 2: Convert to BSON (pure Rust, no GIL needed)
            let filter_doc = items_to_bson_document(&filter_items)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            validate_query_if_enabled(&filter_doc)?;

            let update_doc = items_to_bson_document(&update_items)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            // Check if update already has operators, if not wrap in $set
            let final_update = if update_doc.keys().any(|k| k.starts_with('$')) {
                update_doc
            } else {
                doc! { "$set": update_doc }
            };

            let result = collection
                .update_many(filter_doc, final_update)
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(result.modified_count)
        })
    }

    /// Delete a single document matching the filter
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///
    /// Returns:
    ///     Number of documents deleted (0 or 1)
    #[staticmethod]
    fn delete_one<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Phase 1: Extract Python data (GIL held, minimal work)
        let config = get_config();
        let context = ConversionContext {
            security_config: config,
            max_depth: 100,
            max_size: 16 * 1024 * 1024,
            strict_types: true,
        };

        let filter_items = extract_dict_items(py, filter, &context)?;

        future_into_py(py, async move {
            // Phase 2: Convert to BSON (pure Rust, no GIL)
            let filter_doc = items_to_bson_document(&filter_items)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            validate_query_if_enabled(&filter_doc)?;

            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .delete_one(filter_doc)
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(result.deleted_count)
        })
    }

    /// Find documents and return as typed Python Document instances
    ///
    /// This is an optimized version that creates Python objects directly in Rust,
    /// avoiding the intermediate dict conversion step.
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     document_class: The Python Document class to instantiate
    ///     filter: Query filter as a dict (optional)
    ///     sort: Sort specification as a dict (optional)
    ///     skip: Number of documents to skip (optional)
    ///     limit: Maximum documents to return (optional)
    ///
    /// Returns:
    ///     A list of Document instances (typed)
    #[staticmethod]
    #[pyo3(signature = (collection_name, document_class, filter=None, sort=None, skip=None, limit=None))]
    fn find_as_documents<'py>(
        py: Python<'py>,
        collection_name: String,
        document_class: Bound<'py, PyAny>,
        filter: Option<&Bound<'_, PyDict>>,
        sort: Option<&Bound<'_, PyDict>>,
        skip: Option<u64>,
        limit: Option<i64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = match filter {
            Some(dict) => py_dict_to_bson(py, dict)?,
            None => doc! {},
        };
        let sort_doc = match sort {
            Some(dict) => Some(py_dict_to_bson(py, dict)?),
            None => None,
        };

        // Clone the class reference for use in async block
        let doc_class = document_class.unbind();

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            // Build find options
            let mut find_options = mongodb::options::FindOptions::default();
            if let Some(sort) = sort_doc {
                find_options.sort = Some(sort);
            }
            if let Some(skip_val) = skip {
                find_options.skip = Some(skip_val);
            }
            if let Some(limit_val) = limit {
                find_options.limit = Some(limit_val);
            }

            let cursor = collection
                .find(filter_doc)
                .with_options(find_options)
                .await
                .map_err(sanitize_mongodb_error)?;

            let docs: Vec<BsonDocument> = cursor
                .try_collect()
                .await
                .map_err(sanitize_mongodb_error)?;

            // Phase 1: Convert BSON to intermediate (no GIL needed, can parallelize)
            let intermediate: Vec<(Option<String>, Vec<(String, ExtractedValue)>)> =
                if docs.len() >= PARALLEL_THRESHOLD {
                    // Parallel conversion for large result sets
                    docs.into_par_iter()
                        .map(|bson_doc| {
                            let id_str = bson_doc
                                .get("_id")
                                .and_then(|v| v.as_object_id())
                                .map(|oid| oid.to_hex());

                            let fields: Vec<(String, ExtractedValue)> = bson_doc
                                .iter()
                                .filter(|(k, _)| *k != "_id")
                                .map(|(k, v)| (k.clone(), bson_to_extracted(v)))
                                .collect();

                            (id_str, fields)
                        })
                        .collect()
                } else {
                    // Sequential for small result sets
                    docs.into_iter()
                        .map(|bson_doc| {
                            let id_str = bson_doc
                                .get("_id")
                                .and_then(|v| v.as_object_id())
                                .map(|oid| oid.to_hex());

                            let fields: Vec<(String, ExtractedValue)> = bson_doc
                                .iter()
                                .filter(|(k, _)| *k != "_id")
                                .map(|(k, v)| (k.clone(), bson_to_extracted(v)))
                                .collect();

                            (id_str, fields)
                        })
                        .collect()
                };

            // Phase 2: Create Python objects (requires GIL)
            Python::with_gil(|py| {
                let doc_class = doc_class.bind(py);
                let mut results: Vec<PyObject> = Vec::with_capacity(intermediate.len());

                for (id_str, fields) in intermediate {
                    // Convert fields to Python dict
                    let py_dict = PyDict::new(py);
                    for (key, value) in fields {
                        py_dict.set_item(&key, extracted_to_py(py, value)?)?;
                    }

                    // Create instance
                    let kwargs = PyDict::new(py);
                    let instance = doc_class.call((), Some(&kwargs))?;

                    // Set attributes
                    instance.setattr("_id", id_str)?;
                    instance.setattr("_data", py_dict)?;

                    results.push(instance.unbind());
                }

                Ok(results)
            })
        })
    }

    /// Find documents and return as Python dicts (optimized path)
    ///
    /// This function returns raw Python dicts instead of Document instances,
    /// allowing Python to use the fast path (_from_db with validate=False).
    /// This eliminates the overhead of creating Document instances one-by-one
    /// in Rust, resulting in 3-5x faster query performance.
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Optional filter dict
    ///     sort: Optional sort specification
    ///     skip: Optional number of documents to skip
    ///     limit: Optional maximum number of documents to return
    ///
    /// Returns:
    ///     List of Python dicts (raw BSON documents)
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter=None, sort=None, skip=None, limit=None))]
    fn find_as_dicts<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: Option<&Bound<'_, PyDict>>,
        sort: Option<&Bound<'_, PyDict>>,
        skip: Option<u64>,
        limit: Option<i64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = match filter {
            Some(dict) => py_dict_to_bson(py, dict)?,
            None => doc! {},
        };
        let sort_doc = match sort {
            Some(dict) => Some(py_dict_to_bson(py, dict)?),
            None => None,
        };

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            // Build find options
            let mut find_options = mongodb::options::FindOptions::default();
            if let Some(sort) = sort_doc {
                find_options.sort = Some(sort);
            }
            if let Some(skip_val) = skip {
                find_options.skip = Some(skip_val);
            }
            if let Some(limit_val) = limit {
                find_options.limit = Some(limit_val);
            }

            let cursor = collection
                .find(filter_doc)
                .with_options(find_options)
                .await
                .map_err(sanitize_mongodb_error)?;

            let docs: Vec<BsonDocument> = cursor
                .try_collect()
                .await
                .map_err(sanitize_mongodb_error)?;

            // Phase 1: Convert BSON to intermediate (no GIL needed, can parallelize)
            let intermediate: Vec<(Option<String>, Vec<(String, ExtractedValue)>)> =
                if docs.len() >= PARALLEL_THRESHOLD {
                    // Parallel conversion for large result sets
                    docs.into_par_iter()
                        .map(|bson_doc| {
                            let id_str = bson_doc
                                .get("_id")
                                .and_then(|v| v.as_object_id())
                                .map(|oid| oid.to_hex());

                            let fields: Vec<(String, ExtractedValue)> = bson_doc
                                .iter()
                                .filter(|(k, _)| *k != "_id")
                                .map(|(k, v)| (k.clone(), bson_to_extracted(v)))
                                .collect();

                            (id_str, fields)
                        })
                        .collect()
                } else {
                    // Sequential for small result sets
                    docs.into_iter()
                        .map(|bson_doc| {
                            let id_str = bson_doc
                                .get("_id")
                                .and_then(|v| v.as_object_id())
                                .map(|oid| oid.to_hex());

                            let fields: Vec<(String, ExtractedValue)> = bson_doc
                                .iter()
                                .filter(|(k, _)| *k != "_id")
                                .map(|(k, v)| (k.clone(), bson_to_extracted(v)))
                                .collect();

                            (id_str, fields)
                        })
                        .collect()
                };

            // Phase 2: Create Python dicts (simpler than creating Document instances)
            Python::with_gil(|py| {
                let mut results: Vec<PyObject> = Vec::with_capacity(intermediate.len());

                for (id_str, fields) in intermediate {
                    let py_dict = PyDict::new(py);

                    // Add _id to dict
                    if let Some(id) = id_str {
                        py_dict.set_item("_id", id)?;
                    }

                    // Add all fields
                    for (key, value) in fields {
                        py_dict.set_item(&key, extracted_to_py(py, value)?)?;
                    }

                    results.push(py_dict.into());
                }

                Ok(results)
            })
        })
    }

    /// Run an aggregation pipeline
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     pipeline: List of pipeline stages as dicts
    ///
    /// Returns:
    ///     List of result documents as dicts
    #[staticmethod]
    fn aggregate<'py>(
        py: Python<'py>,
        collection_name: String,
        pipeline: &Bound<'_, PyList>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Convert pipeline stages to BSON
        let mut bson_pipeline = Vec::with_capacity(pipeline.len());
        for item in pipeline.iter() {
            if let Ok(dict) = item.downcast::<PyDict>() {
                let stage = py_dict_to_bson(py, dict)?;
                bson_pipeline.push(stage);
            } else {
                return Err(PyValueError::new_err("Pipeline stages must be dicts"));
            }
        }

        // Security: Validate aggregation pipeline for dangerous operators
        for stage in &bson_pipeline {
            validate_query_if_enabled(stage)?;
        }

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let cursor = collection
                .aggregate(bson_pipeline)
                .await
                .map_err(sanitize_mongodb_error)?;

            let docs: Vec<BsonDocument> = cursor
                .try_collect()
                .await
                .map_err(sanitize_mongodb_error)?;

            // Return as list of RustDocument for consistency
            let results: Vec<RustDocument> = docs
                .into_iter()
                .map(|doc| {
                    let id = doc.get("_id").and_then(|v| v.as_object_id());
                    RustDocument {
                        collection_name: collection_name.clone(),
                        data: doc,
                        id,
                    }
                })
                .collect();

            Ok(results)
        })
    }

    // ========== Index Management ==========

    /// Create an index on a collection
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     keys: Dict of field names to sort order (1 for ascending, -1 for descending)
    ///     options: Optional dict with index options (unique, name, sparse, etc.)
    ///
    /// Returns:
    ///     Name of the created index
    #[staticmethod]
    #[pyo3(signature = (collection_name, keys, options=None))]
    fn create_index<'py>(
        py: Python<'py>,
        collection_name: String,
        keys: &Bound<'_, PyDict>,
        options: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let keys_doc = py_dict_to_bson(py, keys)?;

        // Parse options if provided
        let mut index_options = IndexOptions::default();
        if let Some(opts) = options {
            // unique
            if let Some(unique) = opts.get_item("unique")? {
                if let Ok(val) = unique.extract::<bool>() {
                    index_options.unique = Some(val);
                }
            }
            // name
            if let Some(name) = opts.get_item("name")? {
                if let Ok(val) = name.extract::<String>() {
                    index_options.name = Some(val);
                }
            }
            // sparse
            if let Some(sparse) = opts.get_item("sparse")? {
                if let Ok(val) = sparse.extract::<bool>() {
                    index_options.sparse = Some(val);
                }
            }
            // expire_after_seconds (TTL index)
            if let Some(expire) = opts.get_item("expire_after_seconds")? {
                if let Ok(val) = expire.extract::<i64>() {
                    index_options.expire_after = Some(std::time::Duration::from_secs(val as u64));
                }
            }
            // background (deprecated but still supported)
            if let Some(bg) = opts.get_item("background")? {
                if let Ok(val) = bg.extract::<bool>() {
                    index_options.background = Some(val);
                }
            }
        }

        let index = IndexModel::builder()
            .keys(keys_doc)
            .options(index_options)
            .build();

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .create_index(index)
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(result.index_name)
        })
    }

    /// Create multiple indexes on a collection
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     indexes: List of dicts, each with 'keys' and optional 'options'
    ///
    /// Returns:
    ///     List of created index names
    #[staticmethod]
    fn create_indexes<'py>(
        py: Python<'py>,
        collection_name: String,
        indexes: &Bound<'_, PyList>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        let mut index_models = Vec::with_capacity(indexes.len());
        for item in indexes.iter() {
            if let Ok(dict) = item.downcast::<PyDict>() {
                // Get keys
                let keys = dict.get_item("keys")?.ok_or_else(|| {
                    PyValueError::new_err("Each index must have 'keys' dict")
                })?;
                let keys_dict = keys.downcast::<PyDict>().map_err(|_| {
                    PyValueError::new_err("'keys' must be a dict")
                })?;
                let keys_doc = py_dict_to_bson(py, keys_dict)?;

                // Parse options
                let mut index_options = IndexOptions::default();
                if let Some(opts) = dict.get_item("options")? {
                    if let Ok(opts_dict) = opts.downcast::<PyDict>() {
                        if let Some(unique) = opts_dict.get_item("unique")? {
                            if let Ok(val) = unique.extract::<bool>() {
                                index_options.unique = Some(val);
                            }
                        }
                        if let Some(name) = opts_dict.get_item("name")? {
                            if let Ok(val) = name.extract::<String>() {
                                index_options.name = Some(val);
                            }
                        }
                        if let Some(sparse) = opts_dict.get_item("sparse")? {
                            if let Ok(val) = sparse.extract::<bool>() {
                                index_options.sparse = Some(val);
                            }
                        }
                        if let Some(expire) = opts_dict.get_item("expire_after_seconds")? {
                            if let Ok(val) = expire.extract::<i64>() {
                                index_options.expire_after = Some(std::time::Duration::from_secs(val as u64));
                            }
                        }
                    }
                }

                let index = IndexModel::builder()
                    .keys(keys_doc)
                    .options(index_options)
                    .build();
                index_models.push(index);
            } else {
                return Err(PyValueError::new_err("Each index must be a dict"));
            }
        }

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let result = collection
                .create_indexes(index_models)
                .await
                .map_err(sanitize_mongodb_error)?;

            let names: Vec<String> = result.index_names;
            Ok(names)
        })
    }

    /// List all indexes on a collection
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///
    /// Returns:
    ///     List of index documents
    #[staticmethod]
    fn list_indexes<'py>(
        py: Python<'py>,
        collection_name: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let cursor = collection
                .list_indexes()
                .await
                .map_err(sanitize_mongodb_error)?;

            let indexes: Vec<mongodb::IndexModel> = cursor
                .try_collect()
                .await
                .map_err(sanitize_mongodb_error)?;

            // Convert to list of index info structs
            let mut result: Vec<IndexInfo> = Vec::new();
            for index in indexes {
                let mut info = IndexInfo {
                    key: index.keys,
                    name: None,
                    unique: None,
                    sparse: None,
                    expire_after_seconds: None,
                    background: None,
                };

                if let Some(opts) = index.options {
                    info.name = opts.name;
                    info.unique = opts.unique;
                    info.sparse = opts.sparse;
                    info.expire_after_seconds = opts.expire_after.map(|d| d.as_secs() as i64);
                    info.background = opts.background;
                }

                result.push(info);
            }

            Ok(result)
        })
    }

    /// Drop an index from a collection
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     index_name: Name of the index to drop
    ///
    /// Returns:
    ///     None
    #[staticmethod]
    fn drop_index<'py>(
        py: Python<'py>,
        collection_name: String,
        index_name: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            collection
                .drop_index(index_name)
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(())
        })
    }

    /// Drop all indexes from a collection (except _id)
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///
    /// Returns:
    ///     None
    #[staticmethod]
    fn drop_indexes<'py>(
        py: Python<'py>,
        collection_name: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            collection
                .drop_indexes()
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(())
        })
    }

    // ========== Collection Management ==========

    /// Create a collection with options (including time-series)
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     options: Optional dict with collection options including:
    ///         - timeseries: Dict with timeField, metaField, granularity
    ///         - expireAfterSeconds: TTL for automatic document deletion
    ///
    /// Returns:
    ///     True if collection was created, False if it already exists
    #[staticmethod]
    #[pyo3(signature = (collection_name, options=None))]
    fn create_collection<'py>(
        py: Python<'py>,
        collection_name: String,
        options: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Parse options if provided
        let options_doc = if let Some(opts) = options {
            Some(py_dict_to_bson(py, opts)?)
        } else {
            None
        };

        future_into_py(py, async move {
            let db = conn.database();

            // Check if collection already exists
            let collections = db.list_collection_names().await
                .map_err(sanitize_mongodb_error)?;

            if collections.contains(&validated_name) {
                return Ok(false);
            }

            // Parse options and create collection
            if let Some(opts) = options_doc {
                // Handle time-series configuration
                if let Some(ts) = opts.get("timeseries").and_then(|v| v.as_document()) {
                    // Required: timeField
                    let time_field = ts.get_str("timeField")
                        .map_err(|_| PyValueError::new_err("timeseries.timeField is required"))?
                        .to_string();

                    // Build time-series options using builder
                    // We need to handle different combinations due to typed builder
                    let meta_field = ts.get_str("metaField").ok();
                    let granularity = ts.get_str("granularity").ok();

                    let ts_built = match (meta_field, granularity) {
                        (Some(meta), Some(gran_str)) => {
                            let gran = match gran_str {
                                "seconds" => mongodb::options::TimeseriesGranularity::Seconds,
                                "minutes" => mongodb::options::TimeseriesGranularity::Minutes,
                                "hours" => mongodb::options::TimeseriesGranularity::Hours,
                                _ => return Err(PyValueError::new_err(format!(
                                    "Invalid granularity: {}. Must be 'seconds', 'minutes', or 'hours'",
                                    gran_str
                                ))),
                            };
                            mongodb::options::TimeseriesOptions::builder()
                                .time_field(time_field)
                                .meta_field(meta.to_string())
                                .granularity(gran)
                                .build()
                        }
                        (Some(meta), None) => {
                            mongodb::options::TimeseriesOptions::builder()
                                .time_field(time_field)
                                .meta_field(meta.to_string())
                                .build()
                        }
                        (None, Some(gran_str)) => {
                            let gran = match gran_str {
                                "seconds" => mongodb::options::TimeseriesGranularity::Seconds,
                                "minutes" => mongodb::options::TimeseriesGranularity::Minutes,
                                "hours" => mongodb::options::TimeseriesGranularity::Hours,
                                _ => return Err(PyValueError::new_err(format!(
                                    "Invalid granularity: {}. Must be 'seconds', 'minutes', or 'hours'",
                                    gran_str
                                ))),
                            };
                            mongodb::options::TimeseriesOptions::builder()
                                .time_field(time_field)
                                .granularity(gran)
                                .build()
                        }
                        (None, None) => {
                            mongodb::options::TimeseriesOptions::builder()
                                .time_field(time_field)
                                .build()
                        }
                    };

                    // Get expireAfterSeconds if present
                    let expire_after = opts.get_i64("expireAfterSeconds").ok()
                        .or_else(|| opts.get_i32("expireAfterSeconds").ok().map(|v| v as i64));

                    // Build create options
                    let create_opts = if let Some(expire) = expire_after {
                        mongodb::options::CreateCollectionOptions::builder()
                            .timeseries(ts_built)
                            .expire_after_seconds(std::time::Duration::from_secs(expire as u64))
                            .build()
                    } else {
                        mongodb::options::CreateCollectionOptions::builder()
                            .timeseries(ts_built)
                            .build()
                    };

                    db.create_collection(&validated_name)
                        .with_options(create_opts)
                        .await
                        .map_err(sanitize_mongodb_error)?;
                } else {
                    // No time-series, create regular collection
                    db.create_collection(&validated_name)
                        .await
                        .map_err(sanitize_mongodb_error)?;
                }
            } else {
                // No options, create regular collection
                db.create_collection(&validated_name)
                    .await
                    .map_err(sanitize_mongodb_error)?;
            }

            Ok(true)
        })
    }

    // ========== Upsert Operations ==========

    /// Update one document with upsert support
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///     update: Update document as a dict
    ///     upsert: If True, insert a new document if no match (default: False)
    ///
    /// Returns:
    ///     Dict with matched_count, modified_count, and upserted_id (if any)
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter, update, upsert=false))]
    fn update_one_with_options<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
        update: &Bound<'_, PyDict>,
        upsert: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = py_dict_to_bson(py, filter)?;
        validate_query_if_enabled(&filter_doc)?;
        let update_doc = py_dict_to_bson(py, update)?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            // Check if update already has operators, if not wrap in $set
            let final_update = if update_doc.keys().any(|k| k.starts_with('$')) {
                update_doc
            } else {
                doc! { "$set": update_doc }
            };

            let options = mongodb::options::UpdateOptions::builder()
                .upsert(upsert)
                .build();

            let result = collection
                .update_one(filter_doc, final_update)
                .with_options(options)
                .await
                .map_err(sanitize_mongodb_error)?;

            // Capture result data for return
            let matched = result.matched_count;
            let modified = result.modified_count;
            let upserted = result.upserted_id.and_then(|v| v.as_object_id().map(|oid| oid.to_hex()));

            Ok(UpdateResult { matched_count: matched, modified_count: modified, upserted_id: upserted })
        })
    }

    /// Update many documents with upsert support
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///     update: Update document as a dict
    ///     upsert: If True, insert a new document if no match (default: False)
    ///
    /// Returns:
    ///     Dict with matched_count, modified_count, and upserted_id (if any)
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter, update, upsert=false))]
    fn update_many_with_options<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
        update: &Bound<'_, PyDict>,
        upsert: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = py_dict_to_bson(py, filter)?;
        validate_query_if_enabled(&filter_doc)?;
        let update_doc = py_dict_to_bson(py, update)?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            // Check if update already has operators, if not wrap in $set
            let final_update = if update_doc.keys().any(|k| k.starts_with('$')) {
                update_doc
            } else {
                doc! { "$set": update_doc }
            };

            let options = mongodb::options::UpdateOptions::builder()
                .upsert(upsert)
                .build();

            let result = collection
                .update_many(filter_doc, final_update)
                .with_options(options)
                .await
                .map_err(sanitize_mongodb_error)?;

            // Capture result data for return
            let matched = result.matched_count;
            let modified = result.modified_count;
            let upserted = result.upserted_id.and_then(|v| v.as_object_id().map(|oid| oid.to_hex()));

            Ok(UpdateResult { matched_count: matched, modified_count: modified, upserted_id: upserted })
        })
    }

    // ========== Replace Operations ==========

    /// Replace a single document
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///     replacement: The replacement document
    ///     upsert: If True, insert if no match (default: False)
    ///
    /// Returns:
    ///     Dict with matched_count, modified_count, and upserted_id (if any)
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter, replacement, upsert=false))]
    fn replace_one<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
        replacement: &Bound<'_, PyDict>,
        upsert: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = py_dict_to_bson(py, filter)?;
        validate_query_if_enabled(&filter_doc)?;
        let mut replacement_doc = py_dict_to_bson(py, replacement)?;

        // Remove _id from replacement if present (MongoDB doesn't allow changing _id)
        replacement_doc.remove("_id");

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let options = mongodb::options::ReplaceOptions::builder()
                .upsert(upsert)
                .build();

            let result = collection
                .replace_one(filter_doc, replacement_doc)
                .with_options(options)
                .await
                .map_err(sanitize_mongodb_error)?;

            // Capture result data for return
            let matched = result.matched_count;
            let modified = result.modified_count;
            let upserted = result.upserted_id.and_then(|v| v.as_object_id().map(|oid| oid.to_hex()));

            Ok(UpdateResult { matched_count: matched, modified_count: modified, upserted_id: upserted })
        })
    }

    // ========== Distinct Operations ==========

    /// Get distinct values for a field
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     field: The field to get distinct values for
    ///     filter: Optional query filter
    ///
    /// Returns:
    ///     List of distinct values
    #[staticmethod]
    #[pyo3(signature = (collection_name, field, filter=None))]
    fn distinct<'py>(
        py: Python<'py>,
        collection_name: String,
        field: String,
        filter: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = match filter {
            Some(dict) => py_dict_to_bson(py, dict)?,
            None => doc! {},
        };

        // Security: Validate query for dangerous operators
        validate_query_if_enabled(&filter_doc)?;

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let values = collection
                .distinct(&field, filter_doc)
                .await
                .map_err(sanitize_mongodb_error)?;

            Ok(DistinctResult { values })
        })
    }

    // ========== Find One and Modify ==========

    /// Find one document and update it, returning the original or modified document
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///     update: Update document as a dict
    ///     return_document: "before" or "after" (default: "before")
    ///     upsert: If True, insert if no match (default: False)
    ///     sort: Optional sort specification
    ///
    /// Returns:
    ///     The found document or None
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter, update, return_document="before", upsert=false, sort=None))]
    fn find_one_and_update<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
        update: &Bound<'_, PyDict>,
        return_document: &str,
        upsert: bool,
        sort: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = py_dict_to_bson(py, filter)?;
        validate_query_if_enabled(&filter_doc)?;
        let update_doc = py_dict_to_bson(py, update)?;
        let sort_doc = match sort {
            Some(dict) => Some(py_dict_to_bson(py, dict)?),
            None => None,
        };

        let return_doc = match return_document {
            "after" => mongodb::options::ReturnDocument::After,
            _ => mongodb::options::ReturnDocument::Before,
        };

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            // Check if update already has operators
            let final_update = if update_doc.keys().any(|k| k.starts_with('$')) {
                update_doc
            } else {
                doc! { "$set": update_doc }
            };

            let mut options = mongodb::options::FindOneAndUpdateOptions::builder()
                .return_document(return_doc)
                .upsert(upsert)
                .build();

            if let Some(s) = sort_doc {
                options.sort = Some(s);
            }

            let result = collection
                .find_one_and_update(filter_doc, final_update)
                .with_options(options)
                .await
                .map_err(sanitize_mongodb_error)?;

            match result {
                Some(doc) => {
                    let id = doc.get("_id").and_then(|v| v.as_object_id());
                    Ok(Some(RustDocument {
                        collection_name: validated_name.clone(),
                        data: doc,
                        id,
                    }))
                }
                None => Ok(None),
            }
        })
    }

    /// Find one document and replace it
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///     replacement: The replacement document
    ///     return_document: "before" or "after" (default: "before")
    ///     upsert: If True, insert if no match (default: False)
    ///
    /// Returns:
    ///     The found document or None
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter, replacement, return_document="before", upsert=false))]
    fn find_one_and_replace<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
        replacement: &Bound<'_, PyDict>,
        return_document: &str,
        upsert: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = py_dict_to_bson(py, filter)?;
        validate_query_if_enabled(&filter_doc)?;
        let mut replacement_doc = py_dict_to_bson(py, replacement)?;
        replacement_doc.remove("_id");

        let return_doc = match return_document {
            "after" => mongodb::options::ReturnDocument::After,
            _ => mongodb::options::ReturnDocument::Before,
        };

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let options = mongodb::options::FindOneAndReplaceOptions::builder()
                .return_document(return_doc)
                .upsert(upsert)
                .build();

            let result = collection
                .find_one_and_replace(filter_doc, replacement_doc)
                .with_options(options)
                .await
                .map_err(sanitize_mongodb_error)?;

            match result {
                Some(doc) => {
                    let id = doc.get("_id").and_then(|v| v.as_object_id());
                    Ok(Some(RustDocument {
                        collection_name: validated_name.clone(),
                        data: doc,
                        id,
                    }))
                }
                None => Ok(None),
            }
        })
    }

    /// Find one document and delete it
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     filter: Query filter as a dict
    ///     sort: Optional sort specification
    ///
    /// Returns:
    ///     The deleted document or None
    #[staticmethod]
    #[pyo3(signature = (collection_name, filter, sort=None))]
    fn find_one_and_delete<'py>(
        py: Python<'py>,
        collection_name: String,
        filter: &Bound<'_, PyDict>,
        sort: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;
        let filter_doc = py_dict_to_bson(py, filter)?;
        validate_query_if_enabled(&filter_doc)?;
        let sort_doc = match sort {
            Some(dict) => Some(py_dict_to_bson(py, dict)?),
            None => None,
        };

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let mut options = mongodb::options::FindOneAndDeleteOptions::default();
            if let Some(s) = sort_doc {
                options.sort = Some(s);
            }

            let result = collection
                .find_one_and_delete(filter_doc)
                .with_options(options)
                .await
                .map_err(sanitize_mongodb_error)?;

            match result {
                Some(doc) => {
                    let id = doc.get("_id").and_then(|v| v.as_object_id());
                    Ok(Some(RustDocument {
                        collection_name: validated_name.clone(),
                        data: doc,
                        id,
                    }))
                }
                None => Ok(None),
            }
        })
    }

    // ========== Bulk Write Operations ==========

    /// Execute bulk write operations
    ///
    /// This method executes operations individually for compatibility with MongoDB <8.0.
    /// For ordered operations, it stops on first error. For unordered, it continues.
    ///
    /// Args:
    ///     collection_name: Name of the MongoDB collection
    ///     operations: List of operation dicts with 'op' key
    ///     ordered: If True, stop on first error (default: True)
    ///
    /// Returns:
    ///     Dict with inserted_count, matched_count, modified_count, deleted_count, upserted_count
    #[staticmethod]
    #[pyo3(signature = (collection_name, operations, ordered=true))]
    fn bulk_write<'py>(
        py: Python<'py>,
        collection_name: String,
        operations: &Bound<'_, PyList>,
        ordered: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Security: Validate collection name
        let validated_name = validate_collection_name(&collection_name)?.into_string();

        let conn = get_connection()?;

        // Phase 1: Extract Python operations (GIL held, minimal work)
        let config = get_config();
        let extracted_ops: Vec<ExtractedBulkOp> = {
            let mut result = Vec::with_capacity(operations.len());

            for item in operations.iter() {
                let dict = item.downcast::<PyDict>().map_err(|_| {
                    PyValueError::new_err("Each operation must be a dict")
                })?;

                let op_type: String = dict
                    .get_item("op")?
                    .ok_or_else(|| PyValueError::new_err("Operation must have 'op' key"))?
                    .extract()?;

                let extracted_op = match op_type.as_str() {
                    "insert_one" => {
                        // Two-step to avoid temporary value borrow issues
                        let doc_item = dict
                            .get_item("document")?
                            .ok_or_else(|| PyValueError::new_err("insert_one requires 'document'"))?;
                        let doc_dict = doc_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("document must be a dict"))?;
                        ExtractedBulkOp::InsertOne {
                            document: extract_dict_fields(py, doc_dict, &config)?,
                        }
                    }
                    "update_one" => {
                        // Two-step to avoid temporary value borrow issues
                        let filter_item = dict
                            .get_item("filter")?
                            .ok_or_else(|| PyValueError::new_err("update requires 'filter'"))?;
                        let filter_dict = filter_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("filter must be a dict"))?;

                        let update_item = dict
                            .get_item("update")?
                            .ok_or_else(|| PyValueError::new_err("update requires 'update'"))?;
                        let update_dict = update_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("update must be a dict"))?;

                        let upsert: bool = dict
                            .get_item("upsert")?
                            .map(|v| v.extract().unwrap_or(false))
                            .unwrap_or(false);

                        ExtractedBulkOp::UpdateOne {
                            filter: extract_dict_fields(py, filter_dict, &config)?,
                            update: extract_dict_fields(py, update_dict, &config)?,
                            upsert,
                        }
                    }
                    "update_many" => {
                        // Two-step to avoid temporary value borrow issues
                        let filter_item = dict
                            .get_item("filter")?
                            .ok_or_else(|| PyValueError::new_err("update requires 'filter'"))?;
                        let filter_dict = filter_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("filter must be a dict"))?;

                        let update_item = dict
                            .get_item("update")?
                            .ok_or_else(|| PyValueError::new_err("update requires 'update'"))?;
                        let update_dict = update_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("update must be a dict"))?;

                        let upsert: bool = dict
                            .get_item("upsert")?
                            .map(|v| v.extract().unwrap_or(false))
                            .unwrap_or(false);

                        ExtractedBulkOp::UpdateMany {
                            filter: extract_dict_fields(py, filter_dict, &config)?,
                            update: extract_dict_fields(py, update_dict, &config)?,
                            upsert,
                        }
                    }
                    "delete_one" => {
                        // Two-step to avoid temporary value borrow issues
                        let filter_item = dict
                            .get_item("filter")?
                            .ok_or_else(|| PyValueError::new_err("delete requires 'filter'"))?;
                        let filter_dict = filter_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("filter must be a dict"))?;

                        ExtractedBulkOp::DeleteOne {
                            filter: extract_dict_fields(py, filter_dict, &config)?,
                        }
                    }
                    "delete_many" => {
                        // Two-step to avoid temporary value borrow issues
                        let filter_item = dict
                            .get_item("filter")?
                            .ok_or_else(|| PyValueError::new_err("delete requires 'filter'"))?;
                        let filter_dict = filter_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("filter must be a dict"))?;

                        ExtractedBulkOp::DeleteMany {
                            filter: extract_dict_fields(py, filter_dict, &config)?,
                        }
                    }
                    "replace_one" => {
                        // Two-step to avoid temporary value borrow issues
                        let filter_item = dict
                            .get_item("filter")?
                            .ok_or_else(|| PyValueError::new_err("replace_one requires 'filter'"))?;
                        let filter_dict = filter_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("filter must be a dict"))?;

                        let replacement_item = dict
                            .get_item("replacement")?
                            .ok_or_else(|| PyValueError::new_err("replace_one requires 'replacement'"))?;
                        let replacement_dict = replacement_item
                            .downcast::<PyDict>()
                            .map_err(|_| PyValueError::new_err("replacement must be a dict"))?;

                        let upsert: bool = dict
                            .get_item("upsert")?
                            .map(|v| v.extract().unwrap_or(false))
                            .unwrap_or(false);

                        ExtractedBulkOp::ReplaceOne {
                            filter: extract_dict_fields(py, filter_dict, &config)?,
                            replacement: extract_dict_fields(py, replacement_dict, &config)?,
                            upsert,
                        }
                    }
                    _ => {
                        return Err(PyValueError::new_err(format!("Unknown operation: {}", op_type)));
                    }
                };

                result.push(extracted_op);
            }
            result
        };

        // Phase 2: Convert to BSON (GIL released, can parallelize)
        let parsed_ops: Vec<(String, BsonDocument, Option<BsonDocument>, bool)> = py.allow_threads(|| {
            if extracted_ops.len() >= PARALLEL_THRESHOLD {
                extracted_ops
                    .into_par_iter()
                    .map(extracted_bulk_op_to_tuple)
                    .collect()
            } else {
                extracted_ops
                    .into_iter()
                    .map(extracted_bulk_op_to_tuple)
                    .collect()
            }
        });

        future_into_py(py, async move {
            let db = conn.database();
            let collection = db.collection::<BsonDocument>(&validated_name);

            let mut inserted_count: i64 = 0;
            let mut matched_count: i64 = 0;
            let mut modified_count: i64 = 0;
            let mut deleted_count: i64 = 0;
            let mut upserted_count: i64 = 0;
            let mut upserted_ids: std::collections::HashMap<i64, String> = std::collections::HashMap::new();

            for (idx, (op_type, doc1, doc2, upsert)) in parsed_ops.into_iter().enumerate() {
                let result = match op_type.as_str() {
                    "insert_one" => {
                        match collection.insert_one(doc1).await {
                            Ok(_) => {
                                inserted_count += 1;
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    "update_one" => {
                        let options = mongodb::options::UpdateOptions::builder()
                            .upsert(upsert)
                            .build();
                        match collection.update_one(doc1, doc2.unwrap()).with_options(options).await {
                            Ok(result) => {
                                matched_count += result.matched_count as i64;
                                modified_count += result.modified_count as i64;
                                if let Some(id) = result.upserted_id {
                                    upserted_count += 1;
                                    if let Some(oid) = id.as_object_id() {
                                        upserted_ids.insert(idx as i64, oid.to_hex());
                                    }
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    "update_many" => {
                        let options = mongodb::options::UpdateOptions::builder()
                            .upsert(upsert)
                            .build();
                        match collection.update_many(doc1, doc2.unwrap()).with_options(options).await {
                            Ok(result) => {
                                matched_count += result.matched_count as i64;
                                modified_count += result.modified_count as i64;
                                if let Some(id) = result.upserted_id {
                                    upserted_count += 1;
                                    if let Some(oid) = id.as_object_id() {
                                        upserted_ids.insert(idx as i64, oid.to_hex());
                                    }
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    "delete_one" => {
                        match collection.delete_one(doc1).await {
                            Ok(result) => {
                                deleted_count += result.deleted_count as i64;
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    "delete_many" => {
                        match collection.delete_many(doc1).await {
                            Ok(result) => {
                                deleted_count += result.deleted_count as i64;
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    "replace_one" => {
                        let options = mongodb::options::ReplaceOptions::builder()
                            .upsert(upsert)
                            .build();
                        match collection.replace_one(doc1, doc2.unwrap()).with_options(options).await {
                            Ok(result) => {
                                matched_count += result.matched_count as i64;
                                modified_count += result.modified_count as i64;
                                if let Some(id) = result.upserted_id {
                                    upserted_count += 1;
                                    if let Some(oid) = id.as_object_id() {
                                        upserted_ids.insert(idx as i64, oid.to_hex());
                                    }
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    _ => Ok(()),
                };

                // In ordered mode, stop on first error
                if ordered {
                    result.map_err(sanitize_mongodb_error)?;
                }
                // In unordered mode, continue with remaining operations (ignore errors)
            }

            Ok(BulkWriteResultWrapper {
                inserted_count,
                matched_count,
                modified_count,
                deleted_count,
                upserted_count,
                upserted_ids,
            })
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "Document(collection='{}', id={})",
            self.collection_name,
            self.id.map_or("None".to_string(), |id| format!("'{}'", id.to_hex()))
        )
    }
}

/// Register the mongodb module
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(is_connected, m)?)?;
    m.add_function(wrap_pyfunction!(close, m)?)?;
    m.add_function(wrap_pyfunction!(reset, m)?)?;
    m.add_function(wrap_pyfunction!(available_features, m)?)?;
    m.add_class::<RustDocument>()?;

    // Add module docstring
    m.add("__doc__", "MongoDB ORM module with Beanie compatibility")?;

    Ok(())
}
