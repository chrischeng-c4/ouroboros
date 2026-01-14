"""
ouroboros: High-performance Rust-powered Python platform.

Features:
    - MongoDB ORM (Beanie-compatible)
    - PostgreSQL ORM
    - HTTP Client
    - Task Queue
    - KV Store
    - All with Rust backend for maximum performance

Quick Start:
    >>> import ouroboros as ob
    >>> from ouroboros.mongodb import Document, init
    >>>
    >>> await init("mongodb://localhost:27017/mydb")
    >>>
    >>> class User(Document):
    ...     email: str
    ...     name: str
    ...
    >>> user = await User.find_one(User.email == "alice@example.com")
"""

__version__ = "0.1.0"

# Import the Rust extension first (the .so file in this directory)
# This provides: mongodb, postgres, http, kv, api, test submodules
# Note: The .so file is named 'ouroboros.abi3.so' and can be imported as '.ouroboros'

# Import sub-packages (Python wrappers around Rust modules)
from . import mongodb
from . import http
from . import postgres
from . import test

# Import KV module if available (feature-gated)
try:
    from . import kv
except ImportError:
    kv = None  # KV feature not enabled

# Import PyLoop module if available (feature-gated)
try:
    from .ouroboros import _pyloop
except (ImportError, AttributeError):
    _pyloop = None  # PyLoop feature not enabled

# Re-export commonly used classes from mongodb for convenience/backward compatibility
from .mongodb import (
    Document, Settings, EmbeddedDocument,
    Field, FieldProxy, QueryExpr, merge_filters, text_search, TextSearch, escape_regex,
    QueryBuilder, AggregationBuilder,
    init, is_connected, close, reset,
    # Actions
    before_event, after_event, Insert, Replace, Save, Delete, ValidateOnSave, EventType,
    # Bulk
    BulkOperation, UpdateOne, UpdateMany, InsertOne, DeleteOne, DeleteMany, ReplaceOne, BulkWriteResult,
    # Types
    PydanticObjectId, Indexed, IndexModelField, get_index_fields,
    # Links
    Link, BackLink, WriteRules, DeleteRules, get_link_fields,
    # Transactions
    Session, Transaction, start_session, TransactionNotSupportedError,
    # Time-series
    TimeSeriesConfig, Granularity,
    # Migrations
    Migration, MigrationHistory, IterativeMigration, FreeFallMigration, iterative_migration, free_fall_migration, run_migrations, get_pending_migrations, get_applied_migrations, get_migration_status,
    # Constraints
    Constraint, MinLen, MaxLen, Min, Max, Email, Url,
)

__all__ = [
    # Version
    "__version__",
    # Modules
    "mongodb",
    "http",
    "postgres",
    "test",
    "kv",
    # Connection
    "init",
    "is_connected",
    "close",
    "reset",
    # Core
    "Document",
    "Settings",
    "EmbeddedDocument",
    # Fields
    "Field",
    "FieldProxy",
    "QueryExpr",
    "merge_filters",
    "text_search",
    "TextSearch",
    "escape_regex",
    # Query
    "QueryBuilder",
    "AggregationBuilder",
    # Actions
    "before_event",
    "after_event",
    "Insert",
    "Replace",
    "Save",
    "Delete",
    "ValidateOnSave",
    "EventType",
    # Bulk Operations
    "BulkOperation",
    "UpdateOne",
    "UpdateMany",
    "InsertOne",
    "DeleteOne",
    "DeleteMany",
    "ReplaceOne",
    "BulkWriteResult",
    # Type Support
    "PydanticObjectId",
    "Indexed",
    "IndexModelField",
    "get_index_fields",
    # Document Relations
    "Link",
    "BackLink",
    "WriteRules",
    "DeleteRules",
    "get_link_fields",
    # Transactions
    "Session",
    "Transaction",
    "start_session",
    "TransactionNotSupportedError",
    # Time-series Collections
    "TimeSeriesConfig",
    "Granularity",
    # Migrations
    "Migration",
    "MigrationHistory",
    "IterativeMigration",
    "FreeFallMigration",
    "iterative_migration",
    "free_fall_migration",
    "run_migrations",
    "get_pending_migrations",
    "get_applied_migrations",
    "get_migration_status",
    # Constraints
    "Constraint",
    "MinLen",
    "MaxLen",
    "Min",
    "Max",
    "Email",
    "Url",
]

# Re-export KvClient if available
try:
    from .kv import KvClient
    __all__.append("KvClient")
except ImportError:
    pass  # KV feature not enabled