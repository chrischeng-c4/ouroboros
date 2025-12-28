"""PostgreSQL ORM for data-bridge."""

from .table import Table
from .columns import Column, ColumnProxy
from .query import QueryBuilder
from .connection import (
    init, close, is_connected, execute,
    list_tables, table_exists, get_columns, get_indexes, inspect_table,
    migration_init, migration_status, migration_apply,
    migration_rollback, migration_create
)
from .transactions import pg_transaction, Transaction
from .migrations import Migration, run_migrations, get_migration_status

__all__ = [
    # Base class
    "Table",
    # Fields
    "Column",
    "ColumnProxy",
    # Query
    "QueryBuilder",
    # Connection
    "init",
    "close",
    "is_connected",
    "execute",
    # Schema Introspection
    "list_tables",
    "table_exists",
    "get_columns",
    "get_indexes",
    "inspect_table",
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
]
