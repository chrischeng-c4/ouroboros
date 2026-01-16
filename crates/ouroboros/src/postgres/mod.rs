//! PostgreSQL module for Python bindings
//!
//! This module provides Python bindings for PostgreSQL operations using PyO3.
//! All SQL serialization/deserialization happens in Rust for maximum performance.

use pyo3::prelude::*;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use ouroboros_postgres::Connection;

// Global connection pool using RwLock for close/reset support
pub(crate) static PG_POOL: StdRwLock<Option<Arc<Connection>>> = StdRwLock::new(None);

// Sub-modules
mod safety;
mod wrappers;
mod conversion;
mod connection;
mod crud;
mod relations;
mod transaction;
mod schema;
mod migration;
mod query_functions;

#[cfg(test)]
mod tests;

// Re-exports for use by other modules
pub(crate) use wrappers::{RowWrapper, OptionalRowWrapper, RowsWrapper};

/// Register PostgreSQL module functions with Python
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register classes
    m.add_class::<transaction::PyTransaction>()?;

    // Connection functions
    m.add_function(wrap_pyfunction!(connection::init, m)?)?;
    m.add_function(wrap_pyfunction!(connection::close, m)?)?;
    m.add_function(wrap_pyfunction!(connection::is_connected, m)?)?;

    // CRUD functions
    m.add_function(wrap_pyfunction!(crud::insert_one, m)?)?;
    m.add_function(wrap_pyfunction!(crud::insert_many, m)?)?;
    m.add_function(wrap_pyfunction!(crud::upsert_one, m)?)?;
    m.add_function(wrap_pyfunction!(crud::upsert_many, m)?)?;
    m.add_function(wrap_pyfunction!(crud::fetch_one, m)?)?;
    m.add_function(wrap_pyfunction!(crud::fetch_all, m)?)?;
    m.add_function(wrap_pyfunction!(crud::fetch_one_with_relations, m)?)?;
    m.add_function(wrap_pyfunction!(crud::fetch_one_eager, m)?)?;
    m.add_function(wrap_pyfunction!(crud::fetch_many_with_relations, m)?)?;
    m.add_function(wrap_pyfunction!(crud::update_one, m)?)?;
    m.add_function(wrap_pyfunction!(crud::update_many, m)?)?;
    m.add_function(wrap_pyfunction!(crud::delete_one, m)?)?;
    m.add_function(wrap_pyfunction!(crud::delete_many, m)?)?;
    m.add_function(wrap_pyfunction!(crud::delete_with_cascade, m)?)?;
    m.add_function(wrap_pyfunction!(crud::delete_checked, m)?)?;
    m.add_function(wrap_pyfunction!(crud::get_backreferences, m)?)?;
    m.add_function(wrap_pyfunction!(crud::count, m)?)?;

    // Relations functions
    m.add_function(wrap_pyfunction!(relations::execute, m)?)?;
    m.add_function(wrap_pyfunction!(relations::m2m_create_join_table, m)?)?;
    m.add_function(wrap_pyfunction!(relations::m2m_add_relation, m)?)?;
    m.add_function(wrap_pyfunction!(relations::m2m_remove_relation, m)?)?;
    m.add_function(wrap_pyfunction!(relations::m2m_clear_relations, m)?)?;
    m.add_function(wrap_pyfunction!(relations::m2m_fetch_related, m)?)?;
    m.add_function(wrap_pyfunction!(relations::m2m_count_related, m)?)?;
    m.add_function(wrap_pyfunction!(relations::m2m_has_relation, m)?)?;
    m.add_function(wrap_pyfunction!(relations::m2m_set_relations, m)?)?;

    // Transaction functions
    m.add_function(wrap_pyfunction!(transaction::begin_transaction, m)?)?;

    // Query functions
    m.add_function(wrap_pyfunction!(query_functions::find_by_foreign_key, m)?)?;
    m.add_function(wrap_pyfunction!(query_functions::find_many, m)?)?;
    m.add_function(wrap_pyfunction!(query_functions::query_aggregate, m)?)?;
    m.add_function(wrap_pyfunction!(query_functions::query_with_cte, m)?)?;

    // Schema introspection functions
    m.add_function(wrap_pyfunction!(schema::list_tables, m)?)?;
    m.add_function(wrap_pyfunction!(schema::table_exists, m)?)?;
    m.add_function(wrap_pyfunction!(schema::get_columns, m)?)?;
    m.add_function(wrap_pyfunction!(schema::get_indexes, m)?)?;
    m.add_function(wrap_pyfunction!(schema::get_foreign_keys, m)?)?;
    m.add_function(wrap_pyfunction!(schema::inspect_table, m)?)?;

    // Migration functions
    m.add_function(wrap_pyfunction!(migration::migration_init, m)?)?;
    m.add_function(wrap_pyfunction!(migration::migration_status, m)?)?;
    m.add_function(wrap_pyfunction!(migration::migration_apply, m)?)?;
    m.add_function(wrap_pyfunction!(migration::migration_rollback, m)?)?;
    m.add_function(wrap_pyfunction!(migration::migration_create, m)?)?;
    m.add_function(wrap_pyfunction!(migration::autogenerate_migration, m)?)?;

    // Add module docstring
    m.add("__doc__", "PostgreSQL ORM module with async support")?;

    Ok(())
}
