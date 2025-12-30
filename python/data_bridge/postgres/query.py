"""
Query builder for chainable PostgreSQL queries.

This module provides a query builder that supports:
- Fluent/chainable API: .order_by().offset().limit().to_list()
- Async execution with Rust backend
- Type-safe query expressions

Example:
    >>> users = await User.find(User.age > 25) \\
    ...     .order_by(-User.created_at) \\
    ...     .offset(10) \\
    ...     .limit(20) \\
    ...     .to_list()
"""

from __future__ import annotations

from typing import Any, Generic, List, Optional, Type, TypeVar, TYPE_CHECKING, Union

from .columns import SqlExpr

if TYPE_CHECKING:
    from .table import Table
    from .columns import ColumnProxy

# Import from Rust engine when available
try:
    from data_bridge.data_bridge import postgres as _engine
except ImportError:
    _engine = None


T = TypeVar("T", bound="Table")


class QueryBuilder(Generic[T]):
    """
    Chainable query builder for PostgreSQL operations.

    Provides a fluent API for building and executing queries.
    All terminal operations (to_list, first, count, exists) are async.

    Example:
        >>> # Find all active users over 25, sorted by creation date
        >>> users = await User.find(User.active == True, User.age > 25) \\
        ...     .order_by(-User.created_at) \\
        ...     .offset(0) \\
        ...     .limit(100) \\
        ...     .to_list()

        >>> # Count matching rows
        >>> count = await User.find(User.status == "active").count()

        >>> # Check existence
        >>> exists = await User.find(User.email == "test@example.com").exists()
    """

    def __init__(
        self,
        model: Type[T],
        filters: tuple,
        _order_by: Optional[List[tuple]] = None,
        _offset: int = 0,
        _limit: int = 0,
        _select: Optional[List[str]] = None,
    ) -> None:
        """
        Initialize query builder.

        Args:
            model: Table model class
            filters: Tuple of SqlExpr or dict filters
            _order_by: Order specification [(column, direction), ...]
            _offset: Number of rows to skip
            _limit: Maximum rows to return (0 = no limit)
            _select: Columns to select (None = all columns)
        """
        self._model = model
        self._filters = filters
        self._order_by_spec = _order_by or []
        self._offset_val = _offset
        self._limit_val = _limit
        self._select_cols = _select

    def _clone(self, **kwargs: Any) -> "QueryBuilder[T]":
        """Create a copy of this builder with updated values."""
        return QueryBuilder(
            model=kwargs.get("model", self._model),
            filters=kwargs.get("filters", self._filters),
            _order_by=kwargs.get("_order_by", self._order_by_spec.copy()),
            _offset=kwargs.get("_offset", self._offset_val),
            _limit=kwargs.get("_limit", self._limit_val),
            _select=kwargs.get("_select", self._select_cols),
        )

    def order_by(self, *fields: Union[ColumnProxy, str]) -> "QueryBuilder[T]":
        """
        Add ordering to the query.

        Args:
            *fields: Column proxies or column names to order by.
                    Prefix with - for descending order.

        Returns:
            New QueryBuilder with ordering applied

        Example:
            >>> # Sort by age ascending
            >>> users = await User.find().order_by(User.age).to_list()
            >>>
            >>> # Sort by age descending
            >>> users = await User.find().order_by(-User.age).to_list()
            >>>
            >>> # Multiple sort fields
            >>> users = await User.find().order_by(-User.created_at, User.name).to_list()
        """
        from .columns import ColumnProxy

        order_spec = []
        for field in fields:
            if isinstance(field, str):
                # String column name
                if field.startswith("-"):
                    order_spec.append((field[1:], "DESC"))
                else:
                    order_spec.append((field, "ASC"))
            elif isinstance(field, ColumnProxy):
                # ColumnProxy
                order_spec.append((field.name, "ASC"))
            elif hasattr(field, "__neg__"):
                # Negated ColumnProxy (-User.age)
                # This is a bit tricky - we need to handle the unary minus
                # For now, just use the name
                order_spec.append((field.name, "DESC"))
            else:
                raise TypeError(f"Invalid order_by field type: {type(field)}")

        new_spec = self._order_by_spec + order_spec
        return self._clone(_order_by=new_spec)

    def offset(self, count: int) -> "QueryBuilder[T]":
        """
        Skip the first N rows.

        Args:
            count: Number of rows to skip

        Returns:
            New QueryBuilder with offset applied

        Example:
            >>> # Skip first 10 rows
            >>> users = await User.find().offset(10).to_list()
            >>>
            >>> # Pagination: page 3, 20 per page
            >>> users = await User.find().offset(40).limit(20).to_list()
        """
        return self._clone(_offset=count)

    def limit(self, count: int) -> "QueryBuilder[T]":
        """
        Limit the number of rows returned.

        Args:
            count: Maximum number of rows to return

        Returns:
            New QueryBuilder with limit applied

        Example:
            >>> # Get first 10 rows
            >>> users = await User.find().limit(10).to_list()
            >>>
            >>> # Pagination: page 1, 20 per page
            >>> users = await User.find().limit(20).to_list()
        """
        return self._clone(_limit=count)

    def select(self, *columns: Union[ColumnProxy, str]) -> "QueryBuilder[T]":
        """
        Select specific columns to return.

        By default, all columns are selected. Use this to limit the columns.

        Args:
            *columns: Column proxies or column names to select

        Returns:
            New QueryBuilder with column selection applied

        Example:
            >>> # Select only email and name columns
            >>> users = await User.find().select(User.email, User.name).to_list()
            >>>
            >>> # Using strings
            >>> users = await User.find().select("email", "name").to_list()
        """
        from .columns import ColumnProxy

        column_names = []
        for col in columns:
            if isinstance(col, str):
                column_names.append(col)
            elif isinstance(col, ColumnProxy):
                column_names.append(col.name)
            else:
                raise TypeError(f"Invalid select column type: {type(col)}")

        return self._clone(_select=column_names)

    async def to_list(self) -> List[T]:
        """
        Execute the query and return all matching rows.

        Returns:
            List of table instances

        Example:
            >>> users = await User.find(User.age > 25).to_list()
            >>> for user in users:
            ...     print(user.name)
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        table_name = self._model.__table_name__()

        # Build SQL query
        where_clause, params = self._build_where_clause()

        # Execute query
        rows = await _engine.find_many(
            table_name,
            where_clause,
            params,
            self._order_by_spec,
            self._offset_val,
            self._limit_val,
            self._select_cols,
        )

        # Convert to model instances
        return [self._model(**row) for row in rows]

    async def first(self) -> Optional[T]:
        """
        Execute the query and return the first matching row.

        Returns:
            Table instance or None if no match

        Example:
            >>> user = await User.find(User.email == "alice@example.com").first()
            >>> if user:
            ...     print(user.name)
        """
        # Use limit(1) and return first result
        result = await self._clone(_limit=1).to_list()
        return result[0] if result else None

    async def count(self) -> int:
        """
        Count the number of matching rows.

        Returns:
            Number of matching rows

        Example:
            >>> total = await User.find().count()
            >>> adults = await User.find(User.age >= 18).count()
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        table_name = self._model.__table_name__()

        # Build SQL query
        where_clause, params = self._build_where_clause()

        # Execute count query
        return await _engine.count(table_name, where_clause, params)

    async def exists(self) -> bool:
        """
        Check if any rows match the query.

        Returns:
            True if at least one row matches, False otherwise

        Example:
            >>> exists = await User.find(User.email == "test@example.com").exists()
            >>> if exists:
            ...     print("Email already registered")
        """
        count = await self.count()
        return count > 0

    def _build_where_clause(self) -> tuple[str, list[Any]]:
        """
        Build WHERE clause from filters.

        Returns:
            Tuple of (where_clause, parameters)
        """
        if not self._filters:
            return ("", [])

        # Convert filters to SQL
        conditions = []
        params = []
        param_index = 1

        for filter_item in self._filters:
            if isinstance(filter_item, SqlExpr):
                sql, filter_params = filter_item.to_sql(param_index)
                conditions.append(sql)
                params.extend(filter_params)
                param_index += len(filter_params)
            elif isinstance(filter_item, dict):
                # Convert dict to SQL conditions
                for key, value in filter_item.items():
                    conditions.append(f"{key} = ${param_index}")
                    params.append(value)
                    param_index += 1
            else:
                raise TypeError(f"Invalid filter type: {type(filter_item)}")

        where_clause = " AND ".join(conditions) if conditions else ""
        return (where_clause, params)
