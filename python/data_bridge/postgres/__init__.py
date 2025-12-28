"""PostgreSQL ORM for data-bridge."""

from .table import Table
from .columns import Column, ColumnProxy
from .query import QueryBuilder
from .connection import init, close, is_connected, execute
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
    # Transactions
    "pg_transaction",
    "Transaction",
    # Migrations
    "Migration",
    "run_migrations",
    "get_migration_status",
]
