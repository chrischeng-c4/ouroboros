"""PostgreSQL ORM for data-bridge."""

from .table import Table
from .columns import Column, ColumnProxy, ForeignKeyProxy, BackReference, BackReferenceQuery, ManyToMany, ManyToManyQuery, create_m2m_join_table
from .query import QueryBuilder
from .relationships import relationship, LoadingStrategy, RelationshipDescriptor
from .options import QueryOption, selectinload, joinedload, noload, raiseload
from .fulltext import FullTextSearch, fts
from .postgis import Point, GeoQuery
from .arrays import ArrayOps
from .connection import (
    init, close, is_connected, execute, query_aggregate, query_with_cte,
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
from .session import Session, IdentityMap, DirtyTracker, UnitOfWork, get_session
from .events import (
    EventType, EventDispatcher, listens_for,
    before_insert, after_insert,
    before_update, after_update,
    before_delete, after_delete,
    before_flush, after_commit,
    AttributeEvents
)
from .telemetry import (
    is_tracing_enabled, get_tracer, get_meter,
    SpanAttributes, MetricNames,
    create_query_span, create_session_span, create_relationship_span,
    add_exception, set_span_result,
    instrument_span, instrument_query, instrument_session,
    ConnectionPoolMetrics, get_connection_pool_metrics
)

__all__ = [
    # Base class
    "Table",
    # Fields
    "Column",
    "ColumnProxy",
    "ForeignKeyProxy",
    "BackReference",
    "BackReferenceQuery",
    "ManyToMany",
    "ManyToManyQuery",
    "create_m2m_join_table",
    # Relationships (Lazy Loading)
    "relationship",
    "LoadingStrategy",
    "RelationshipDescriptor",
    # Query Options (Eager Loading)
    "QueryOption",
    "selectinload",
    "joinedload",
    "noload",
    "raiseload",
    # Query
    "QueryBuilder",
    # Connection
    "init",
    "close",
    "is_connected",
    "execute",
    "query_aggregate",
    "query_with_cte",
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
    # Session Management
    "Session",
    "IdentityMap",
    "DirtyTracker",
    "UnitOfWork",
    "get_session",
    # Event System
    "EventType",
    "EventDispatcher",
    "listens_for",
    "before_insert",
    "after_insert",
    "before_update",
    "after_update",
    "before_delete",
    "after_delete",
    "before_flush",
    "after_commit",
    "AttributeEvents",
    # PostgreSQL Extensions
    "FullTextSearch",
    "fts",
    "Point",
    "GeoQuery",
    "ArrayOps",
    # OpenTelemetry Integration
    "is_tracing_enabled",
    "get_tracer",
    "get_meter",
    "SpanAttributes",
    "MetricNames",
    "create_query_span",
    "create_session_span",
    "create_relationship_span",
    "add_exception",
    "set_span_result",
    "instrument_span",
    "instrument_query",
    "instrument_session",
    "ConnectionPoolMetrics",
    "get_connection_pool_metrics",
]
