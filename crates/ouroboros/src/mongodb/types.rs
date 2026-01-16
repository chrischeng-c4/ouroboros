//! MongoDB types and internal structures.

use bson::{Binary, Bson, Document as BsonDocument, Decimal128};
use bson::oid::ObjectId;
use bson::spec::BinarySubtype;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use pyo3::conversion::IntoPyObject;
use std::str::FromStr;

use super::conversion::bson_to_py;

/// Index information returned by list_indexes
#[derive(Debug, Clone)]
pub(super) struct IndexInfo {
    pub key: BsonDocument,
    pub name: Option<String>,
    pub unique: Option<bool>,
    pub sparse: Option<bool>,
    pub expire_after_seconds: Option<i64>,
    pub background: Option<bool>,
}

/// Update result information
#[derive(Debug, Clone)]
pub(super) struct UpdateResult {
    pub matched_count: u64,
    pub modified_count: u64,
    pub upserted_id: Option<String>,
}

/// Phase timing for profiling MongoDB operations
#[derive(Debug, Clone, Default)]
pub struct OperationTiming {
    /// Time to convert Python filter to BSON (microseconds)
    pub filter_convert_us: u64,
    /// Time for MongoDB network round-trip (microseconds)
    pub network_us: u64,
    /// Time to convert BSON to intermediate representation (microseconds)
    pub bson_to_intermediate_us: u64,
    /// Time to convert intermediate to Python objects (microseconds)
    pub intermediate_to_python_us: u64,
    /// Total operation time (microseconds)
    pub total_us: u64,
    /// Number of documents processed
    pub doc_count: usize,
}

/// Distinct result wrapper
#[derive(Debug, Clone)]
pub(super) struct DistinctResult {
    pub values: Vec<Bson>,
}

/// Bulk write result wrapper
#[derive(Debug, Clone)]
pub(super) struct BulkWriteResultWrapper {
    pub inserted_count: i64,
    pub matched_count: i64,
    pub modified_count: i64,
    pub deleted_count: i64,
    pub upserted_count: i64,
    pub upserted_ids: std::collections::HashMap<i64, String>,
}

/// Intermediate representation for Python values
///
/// This type allows us to extract data from Python while holding the GIL,
/// then convert to BSON without the GIL for better performance.
#[derive(Debug, Clone)]
pub(super) enum ExtractedValue {
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
pub(super) enum ExtractedBulkOp {
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

// =====================
// IntoPyObject implementations
// =====================

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

// =====================
// Conversion helpers for ExtractedValue
// =====================

/// Convert extracted value to BSON (can be called without GIL)
pub(super) fn extracted_to_bson(value: ExtractedValue) -> Bson {
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
pub(super) fn bson_to_extracted(value: &Bson) -> ExtractedValue {
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
pub(super) fn extracted_to_py(py: Python<'_>, value: ExtractedValue) -> PyResult<PyObject> {
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

/// Convert ExtractedBulkOp to tuple format used by bulk_write
pub(super) fn extracted_bulk_op_to_tuple(op: ExtractedBulkOp) -> (String, BsonDocument, Option<BsonDocument>, bool) {
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
