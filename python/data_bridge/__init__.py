"""
data-bridge: High-performance MongoDB ORM with Rust backend.

A Beanie-compatible Python API with all BSON serialization handled in Rust.

Quick Start:
    >>> from data_bridge import Document, Field, init
    >>>
    >>> # Initialize connection
    >>> await init("mongodb://localhost:27017/mydb")
    >>>
    >>> # Define a model
    >>> class User(Document):
    ...     email: str
    ...     name: str
    ...     age: int = 0
    ...
    ...     class Settings:
    ...         name = "users"
    >>>
    >>> # Create and save
    >>> user = User(email="alice@example.com", name="Alice", age=30)
    >>> await user.save()
    >>>
    >>> # Query with type-safe expressions
    >>> user = await User.find_one(User.email == "alice@example.com")
    >>> users = await User.find(User.age > 25).to_list()

Features:
    - Beanie-compatible API for easy migration
    - Type-safe query expressions (User.email == "x")
    - All BSON serialization in Rust for maximum performance
    - Async-first design with asyncio/tokio bridge
    - Chainable query builder with sort, skip, limit

Architecture:
    - Python layer: Provides Beanie-like API, Document base class, QueryBuilder
    - Rust engine: Handles BSON encoding/decoding, MongoDB operations
    - Zero Python byte handling: Data never touches Python heap as raw bytes
"""

__version__ = "0.1.0"

# Import sub-packages
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
    from .data_bridge import _pyloop
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