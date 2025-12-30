"""PostgreSQL ORM for data-bridge."""

from .table import Table
from .columns import Column, ColumnProxy, ForeignKeyProxy, BackReference, BackReferenceQuery
from .query import QueryBuilder
from .connection import (
    init, close, is_connected, execute,
    insert_one, insert_many,
    upsert_one, upsert_many,
    list_tables, table_exists, get_columns, get_indexes, get_foreign_keys, inspect_table,
    get_backreferences,
    find_by_foreign_key,
    fetch_one_with_relations, fetch_one_eager, fetch_many_with_relations,
    delete_with_cascade, delete_checked,
    migration_init, migration_status, migration_apply,
    migration_rollback, migration_create
)
from .transactions import pg_transaction, Transaction
from .migrations import Migration, run_migrations, get_migration_status, autogenerate_migration

__all__ = [
    # Base class
    "Table",
    # Fields
    "Column",
    "ColumnProxy",
    "ForeignKeyProxy",
    "BackReference",
    "BackReferenceQuery",
    # Query
    "QueryBuilder",
    # Connection
    "init",
    "close",
    "is_connected",
    "execute",
    # CRUD Operations
    "insert_one",
    "insert_many",
    "upsert_one",
    "upsert_many",
    # Schema Introspection
    "list_tables",
    "table_exists",
    "get_columns",
    "get_indexes",
    "get_foreign_keys",
    "get_backreferences",
    "inspect_table",
    # Relationships
    "find_by_foreign_key",
    "fetch_one_with_relations",
    "fetch_one_eager",
    "fetch_many_with_relations",
    # Cascade Delete
    "delete_with_cascade",
    "delete_checked",
    # Transactions
    "pg_transaction",
    "Transaction",
    # Migrations (Rust-based)
    "migration_init",
    "migration_status",
    "migration_apply",
    "migration_rollback",
    "migration_create",
    # Migrations (Python-based - legacy)
    "Migration",
    "run_migrations",
    "get_migration_status",
    "autogenerate_migration",
]
