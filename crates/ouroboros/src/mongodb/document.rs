//! MongoDB Document class and operations.

use bson::{doc, oid::ObjectId, Bson, Document as BsonDocument};
use futures::TryStreamExt;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;
use rayon::prelude::*;

use crate::config::get_config;
use crate::error_handling::sanitize_mongodb_error;
use crate::conversion::{
    extract_dict_items, items_to_bson_document,
    bson_to_serializable, serializable_to_py_dict,
    ConversionContext,
};

use super::connection::{get_connection, validate_collection_name, validate_query_if_enabled, PARALLEL_THRESHOLD};
use super::conversion::{py_dict_to_bson, py_to_bson, bson_to_py, bson_doc_to_py_dict, extract_py_value};
use super::types::{ExtractedValue, extracted_to_bson};

/// MongoDB Document class for Python
///
/// This class provides a Python interface to MongoDB documents with full CRUD support.
/// All BSON serialization happens in Rust for maximum performance.
///
/// Example:
///     >>> from ouroboros.mongodb import Document, init
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
                // Optimization: Set batch_size to match limit to reduce round trips
                find_options.batch_size = Some(limit_val as u32);
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

    fn __repr__(&self) -> String {
        format!(
            "Document(collection='{}', id={})",
            self.collection_name,
            self.id.map_or("None".to_string(), |id| format!("'{}'", id.to_hex()))
        )
    }
}
