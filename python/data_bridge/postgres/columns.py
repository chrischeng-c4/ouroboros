"""Column descriptors and SQL expressions for PostgreSQL tables.

This module provides:
- ColumnProxy: Enables User.email == "x" syntax for type-safe SQL queries
- SqlExpr: Represents a single SQL query condition
- Column: Column descriptor with defaults

Example:
    >>> class User(Table):
    ...     email: str
    ...     age: int
    ...
    >>> # These create SqlExpr objects
    >>> User.email == "alice@example.com"
    >>> User.age > 25
    >>> User.name.in_(["Alice", "Bob"])
"""

from __future__ import annotations

from typing import Any, List, Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from .table import Table


class SqlExpr:
    """
    Represents a single SQL query condition.

    Converts to SQL WHERE clause syntax when building queries.

    Example:
        >>> expr = SqlExpr("email", "=", "alice@example.com")
        >>> expr.to_sql()
        ("email = $1", ["alice@example.com"])

        >>> expr = SqlExpr("age", ">", 25)
        >>> expr.to_sql()
        ("age > $1", [25])
    """

    def __init__(self, column: str, op: str, value: Any) -> None:
        """
        Initialize SQL expression.

        Args:
            column: Column name
            op: SQL operator (=, !=, >, >=, <, <=, IN, LIKE, etc.)
            value: Value to compare against
        """
        self.column = column
        self.op = op
        self.value = value

    def to_sql(self, param_index: int = 1) -> tuple[str, list[Any]]:
        """
        Convert to SQL WHERE clause.

        Args:
            param_index: Starting parameter index for placeholders

        Returns:
            Tuple of (sql_string, parameters)
        """
        # Special cases
        if self.op == "IN":
            placeholders = ", ".join(f"${i}" for i in range(param_index, param_index + len(self.value)))
            return (f"{self.column} IN ({placeholders})", list(self.value))
        elif self.op == "BETWEEN":
            return (
                f"{self.column} BETWEEN ${param_index} AND ${param_index + 1}",
                [self.value[0], self.value[1]],
            )
        elif self.op == "IS NULL":
            return (f"{self.column} IS NULL", [])
        elif self.op == "IS NOT NULL":
            return (f"{self.column} IS NOT NULL", [])
        else:
            return (f"{self.column} {self.op} ${param_index}", [self.value])

    def __repr__(self) -> str:
        return f"SqlExpr({self.column!r}, {self.op!r}, {self.value!r})"

    def __and__(self, other: "SqlExpr") -> "SqlExpr":
        """Combine two expressions with AND."""
        if isinstance(other, SqlExpr):
            return SqlExpr("AND", "AND", [self, other])
        raise TypeError(f"Cannot combine SqlExpr with {type(other)}")

    def __or__(self, other: "SqlExpr") -> "SqlExpr":
        """Combine two expressions with OR."""
        if isinstance(other, SqlExpr):
            return SqlExpr("OR", "OR", [self, other])
        raise TypeError(f"Cannot combine SqlExpr with {type(other)}")


class ColumnProxy:
    """
    Column proxy that enables attribute-based query expressions.

    When accessing a column on a Table class (e.g., User.email),
    a ColumnProxy is returned. This proxy overloads comparison operators
    to create SqlExpr objects.

    When accessed on an instance, returns the actual value from _data.

    Example:
        >>> class User(Table):
        ...     email: str
        ...     age: int
        ...
        >>> User.email  # Returns ColumnProxy("email", User)
        >>> User.email == "alice@example.com"  # Returns SqlExpr
        >>> User.age > 25  # Returns SqlExpr
        >>> User.name.in_(["Alice", "Bob"])  # Returns SqlExpr
        >>>
        >>> user = User(email="alice@example.com")
        >>> user.email  # Returns "alice@example.com"
    """

    def __init__(self, name: str, model: Optional[type] = None) -> None:
        """
        Initialize column proxy.

        Args:
            name: Column name
            model: Table class this column belongs to
        """
        self.name = name
        self.model = model

    def __hash__(self) -> int:
        """Make ColumnProxy hashable so it can be used as dict key."""
        return hash((self.name, id(self.model)))

    def __get__(self, obj: Any, objtype: Optional[type] = None) -> Any:
        """
        Descriptor protocol: return value on instance access, self on class access.
        """
        if obj is None:
            # Class access: User.email -> ColumnProxy
            return self
        # Instance access: user.email -> value from _data
        if hasattr(obj, "_data") and self.name in obj._data:
            return obj._data[self.name]
        # Fall back to checking __dict__ or returning None
        return obj.__dict__.get(self.name)

    def __set__(self, obj: Any, value: Any) -> None:
        """
        Descriptor protocol: set value on instance.
        """
        if hasattr(obj, "_data"):
            obj._data[self.name] = value
        else:
            obj.__dict__[self.name] = value

    def __repr__(self) -> str:
        model_name = self.model.__name__ if self.model else "None"
        return f"ColumnProxy({self.name!r}, {model_name})"

    # Comparison operators
    def __eq__(self, other: Any) -> SqlExpr:  # type: ignore[override]
        """Create equality query: User.email == "x" """
        return SqlExpr(self.name, "=", other)

    def __ne__(self, other: Any) -> SqlExpr:  # type: ignore[override]
        """Create not-equal query: User.status != "deleted" """
        return SqlExpr(self.name, "!=", other)

    def __gt__(self, other: Any) -> SqlExpr:
        """Create greater-than query: User.age > 25"""
        return SqlExpr(self.name, ">", other)

    def __ge__(self, other: Any) -> SqlExpr:
        """Create greater-than-or-equal query: User.age >= 25"""
        return SqlExpr(self.name, ">=", other)

    def __lt__(self, other: Any) -> SqlExpr:
        """Create less-than query: User.age < 25"""
        return SqlExpr(self.name, "<", other)

    def __le__(self, other: Any) -> SqlExpr:
        """Create less-than-or-equal query: User.age <= 25"""
        return SqlExpr(self.name, "<=", other)

    # Collection operators
    def in_(self, values: List[Any]) -> SqlExpr:
        """Create IN query: User.status.in_(["active", "pending"])"""
        return SqlExpr(self.name, "IN", values)

    def between(self, low: Any, high: Any) -> SqlExpr:
        """Create BETWEEN query: User.age.between(18, 65)"""
        return SqlExpr(self.name, "BETWEEN", [low, high])

    def is_null(self) -> SqlExpr:
        """Check if column is NULL: User.middle_name.is_null()"""
        return SqlExpr(self.name, "IS NULL", None)

    def is_not_null(self) -> SqlExpr:
        """Check if column is NOT NULL: User.email.is_not_null()"""
        return SqlExpr(self.name, "IS NOT NULL", None)

    # String operators
    def like(self, pattern: str) -> SqlExpr:
        """
        Case-sensitive pattern matching: User.email.like("%@example.com")

        Patterns:
            %: Matches any sequence of characters
            _: Matches any single character

        Example:
            >>> User.email.like("%@example.com")  # Ends with @example.com
            >>> User.name.like("A%")  # Starts with A
        """
        return SqlExpr(self.name, "LIKE", pattern)

    def ilike(self, pattern: str) -> SqlExpr:
        """
        Case-insensitive pattern matching: User.email.ilike("%@EXAMPLE.COM")

        Patterns:
            %: Matches any sequence of characters
            _: Matches any single character

        Example:
            >>> User.email.ilike("%@example.com")  # Case-insensitive
        """
        return SqlExpr(self.name, "ILIKE", pattern)

    def startswith(self, prefix: str) -> SqlExpr:
        """
        Check if column starts with prefix: User.name.startswith("A")

        This is a convenience method that uses LIKE with % wildcard.
        """
        return SqlExpr(self.name, "LIKE", f"{prefix}%")

    def contains(self, substring: str) -> SqlExpr:
        """
        Check if column contains substring: User.bio.contains("python")

        This is a convenience method that uses LIKE with % wildcards.
        """
        return SqlExpr(self.name, "LIKE", f"%{substring}%")


class Column:
    """
    Column descriptor with defaults and constraints.

    This provides Pydantic-style field declaration for Table classes.

    Example:
        >>> from data_bridge.postgres import Table, Column
        >>>
        >>> class User(Table):
        ...     email: str = Column(unique=True)
        ...     age: int = Column(default=0)
        ...     created_at: datetime = Column(default_factory=datetime.utcnow)
        ...
        ...     class Settings:
        ...         table_name = "users"
    """

    def __init__(
        self,
        default: Any = None,
        *,
        default_factory: Optional[callable] = None,
        unique: bool = False,
        index: bool = False,
        nullable: bool = True,
        primary_key: bool = False,
        description: Optional[str] = None,
        foreign_key: Optional[str] = None,
    ) -> None:
        """
        Initialize column with constraints.

        Args:
            default: Default value for the column
            default_factory: Callable that returns default value
            unique: Whether column should have UNIQUE constraint
            index: Whether to create an index on this column
            nullable: Whether column allows NULL values
            primary_key: Whether this is a primary key column
            description: Documentation for this column
            foreign_key: Foreign key reference ("table" or "table.column")
        """
        self.default = default
        self.default_factory = default_factory
        self.unique = unique
        self.index = index
        self.nullable = nullable
        self.primary_key = primary_key
        self.description = description
        self.foreign_key = foreign_key

    def __repr__(self) -> str:
        attrs = []
        if self.default is not None:
            attrs.append(f"default={self.default!r}")
        if self.default_factory is not None:
            attrs.append(f"default_factory={self.default_factory!r}")
        if self.unique:
            attrs.append("unique=True")
        if self.index:
            attrs.append("index=True")
        if not self.nullable:
            attrs.append("nullable=False")
        if self.primary_key:
            attrs.append("primary_key=True")
        if self.foreign_key:
            attrs.append(f"foreign_key={self.foreign_key!r}")
        return f"Column({', '.join(attrs)})"


class ForeignKeyProxy:
    """
    Proxy for foreign key relationships with lazy loading.

    This class enables lazy loading of related objects via foreign keys.
    The related object is only fetched from the database when explicitly
    requested via the fetch() method.

    Example:
        >>> # Assuming Post has foreign key to User
        >>> post = await Post.fetch_one(Post.id == 1)
        >>> print(post.author.ref)  # Get foreign key value without fetching
        123
        >>> author = await post.author.fetch()  # Fetch the related user
        >>> print(author["name"])
        "Alice"
    """

    def __init__(self, target_table: str, foreign_key_column: str, foreign_key_value: Any):
        """
        Initialize foreign key proxy.

        Args:
            target_table: Name of the referenced table
            foreign_key_column: Column name in the referenced table (usually "id")
            foreign_key_value: The foreign key value to query by
        """
        self.target_table = target_table
        self.foreign_key_column = foreign_key_column
        self._foreign_key_value = foreign_key_value
        self._fetched_value = None
        self._is_fetched = False

    async def fetch(self) -> Optional[dict]:
        """
        Fetch the related object from the database.

        If already fetched, returns the cached value without re-querying.

        Returns:
            Dictionary with the related row data, or None if not found

        Example:
            >>> author = await post.author.fetch()
            >>> print(author["name"])
        """
        if self._is_fetched:
            return self._fetched_value

        # Import here to avoid circular dependency
        from data_bridge.postgres import find_by_foreign_key

        result = await find_by_foreign_key(
            self.target_table,
            self.foreign_key_column,
            self._foreign_key_value
        )
        self._fetched_value = result
        self._is_fetched = True
        return result

    @property
    def is_fetched(self) -> bool:
        """
        Check if the related object has been fetched.

        Returns:
            True if fetch() has been called, False otherwise
        """
        return self._is_fetched

    @property
    def ref(self) -> Any:
        """
        Get the foreign key value (ID) without fetching the related object.

        This is useful when you only need the foreign key value itself,
        not the full related object.

        Returns:
            The foreign key value

        Example:
            >>> print(post.author.ref)  # Just the ID, no database query
            123
        """
        return self._foreign_key_value

    @property
    def id(self) -> Any:
        """
        Alias for ref property.

        Returns:
            The foreign key value
        """
        return self._foreign_key_value

    @property
    def column_value(self) -> Any:
        """
        Get the raw column value (foreign key ID).

        Returns:
            The foreign key value
        """
        return self._foreign_key_value

    def __repr__(self) -> str:
        if self._is_fetched:
            return f"ForeignKeyProxy({self.target_table}.{self.foreign_key_column}={self._foreign_key_value}, fetched)"
        return f"ForeignKeyProxy({self.target_table}.{self.foreign_key_column}={self._foreign_key_value}, not fetched)"
