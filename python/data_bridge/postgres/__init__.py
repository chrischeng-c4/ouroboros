"""PostgreSQL ORM for data-bridge."""

from .table import Table
from .columns import Column, ColumnProxy, ForeignKeyProxy, BackReference, BackReferenceQuery, ManyToMany, ManyToManyQuery, create_m2m_join_table
from .query import QueryBuilder
from .query_ext import (
    filter_by, and_, or_, not_, any_, has, aliased,
    QueryFragment, BooleanClause, AliasedClass,
    active_filter, date_range_filter, in_list_filter, null_check_filter
)
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
from .loading import (
    LoadingStrategy, LoadingConfig,
    lazy, joined, subquery, selectinload, noload, raiseload, defer, undefer,
    LazyLoadingProxy, DeferredColumn, RelationshipLoader,
    LazyLoadError, SQLGenerationError
)
from .inheritance import (
    InheritanceType, InheritanceConfig, inheritance,
    SingleTableInheritance, JoinedTableInheritance, ConcreteTableInheritance,
    PolymorphicQueryMixin,
    get_inheritance_type, get_discriminator_column, get_discriminator_value,
    register_polymorphic_class, get_polymorphic_map
)
from .computed import (
    hybrid_property, hybrid_method, column_property, Computed,
    default_factory,
    HybridPropertyDescriptor, HybridMethodDescriptor,
    ColumnPropertyDescriptor, ComputedColumn
)
from .validation import (
    validates, validates_many,
    TypeDecorator,
    coerce_int, coerce_float, coerce_str, coerce_bool, coerce_datetime, coerce_date, coerce_decimal,
    ValidationError,
    ValidatorRegistry,
    AutoCoerceMixin,
    validate_not_empty, validate_email, validate_url,
    validate_min_length, validate_max_length, validate_regex,
    validate_range, validate_min_value, validate_max_value,
    validate_in_list, validate_positive, validate_non_negative
)
from .async_utils import (
    AsyncSession, AsyncSessionFactory,
    run_sync, async_wrap,
    AsyncScoped, get_async_session,
    async_load, async_refresh, async_expire,
    async_stream, AsyncResultIterator,
    greenlet_spawn, AsyncGreenlet, GREENLET_AVAILABLE,
    AsyncEngine
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
    # Query
    "QueryBuilder",
    # Query Extensions
    "filter_by",
    "and_",
    "or_",
    "not_",
    "any_",
    "has",
    "aliased",
    "QueryFragment",
    "BooleanClause",
    "AliasedClass",
    "active_filter",
    "date_range_filter",
    "in_list_filter",
    "null_check_filter",
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
    # Loading Strategies
    "LoadingStrategy",
    "LoadingConfig",
    "lazy",
    "joined",
    "subquery",
    "selectinload",
    "noload",
    "raiseload",
    "defer",
    "undefer",
    "LazyLoadingProxy",
    "DeferredColumn",
    "RelationshipLoader",
    "LazyLoadError",
    "SQLGenerationError",
    # Inheritance Patterns
    "InheritanceType",
    "InheritanceConfig",
    "inheritance",
    "SingleTableInheritance",
    "JoinedTableInheritance",
    "ConcreteTableInheritance",
    "PolymorphicQueryMixin",
    "get_inheritance_type",
    "get_discriminator_column",
    "get_discriminator_value",
    "register_polymorphic_class",
    "get_polymorphic_map",
    # Computed Attributes
    "hybrid_property",
    "hybrid_method",
    "column_property",
    "Computed",
    "default_factory",
    "HybridPropertyDescriptor",
    "HybridMethodDescriptor",
    "ColumnPropertyDescriptor",
    "ComputedColumn",
    # Validation
    "validates",
    "validates_many",
    "TypeDecorator",
    "coerce_int",
    "coerce_float",
    "coerce_str",
    "coerce_bool",
    "coerce_datetime",
    "coerce_date",
    "coerce_decimal",
    "ValidationError",
    "ValidatorRegistry",
    "AutoCoerceMixin",
    "validate_not_empty",
    "validate_email",
    "validate_url",
    "validate_min_length",
    "validate_max_length",
    "validate_regex",
    "validate_range",
    "validate_min_value",
    "validate_max_value",
    "validate_in_list",
    "validate_positive",
    "validate_non_negative",
    # Async Utilities
    "AsyncSession",
    "AsyncSessionFactory",
    "run_sync",
    "async_wrap",
    "AsyncScoped",
    "get_async_session",
    "async_load",
    "async_refresh",
    "async_expire",
    "async_stream",
    "AsyncResultIterator",
    "greenlet_spawn",
    "AsyncGreenlet",
    "GREENLET_AVAILABLE",
    "AsyncEngine",
]
