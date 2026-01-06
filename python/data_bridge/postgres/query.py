"""
Query builder for chainable PostgreSQL queries.

This module provides a query builder that supports:
- Fluent/chainable API: .order_by().offset().limit().to_list()
- Async execution with Rust backend
- Type-safe query expressions
- Aggregation queries: .sum().avg().count_agg().group_by().having().aggregate()
- Common Table Expressions (CTEs): .with_cte().with_cte_raw().from_cte()
- Subqueries: .where_in_subquery().where_exists().where_not_in_subquery().where_not_exists()
- Window functions: .row_number().rank().lag().lead().window_sum().window_avg()

Example:
    >>> # Find and list rows
    >>> users = await User.find(User.age > 25) \\
    ...     .order_by(-User.created_at) \\
    ...     .offset(10) \\
    ...     .limit(20) \\
    ...     .to_list()

    >>> # Aggregate queries
    >>> results = await Order.find(Order.status == "completed") \\
    ...     .sum(Order.amount, "total") \\
    ...     .avg(Order.amount, "average") \\
    ...     .count_agg("count") \\
    ...     .group_by("user_id") \\
    ...     .aggregate()

    >>> # Aggregate with HAVING clause
    >>> results = await Order.find() \\
    ...     .sum(Order.amount, "total") \\
    ...     .group_by("user_id") \\
    ...     .having_sum(Order.amount, ">", 1000) \\
    ...     .aggregate()

    >>> # Using CTEs (Common Table Expressions)
    >>> high_value = Order.find(Order.total > 1000)
    >>> results = await Order.find() \\
    ...     .with_cte("high_value_orders", high_value) \\
    ...     .sum(Order.amount, "total") \\
    ...     .group_by("user_id") \\
    ...     .aggregate()

    >>> # Using subqueries
    >>> order_users = Order.find().select(Order.user_id)
    >>> users = await User.find() \\
    ...     .where_in_subquery(User.id, order_users) \\
    ...     .to_list()

    >>> # Window functions
    >>> results = await Order.find() \\
    ...     .row_number("rank", spec=WindowSpec().order_by(Order.amount, "desc")) \\
    ...     .aggregate()
"""

from __future__ import annotations

from typing import Any, Generic, List, Optional, Type, TypeVar, TYPE_CHECKING, Union

from .columns import SqlExpr
from .telemetry import (
    create_query_span,
    set_span_result,
    add_exception,
    is_tracing_enabled,
)

if TYPE_CHECKING:
    from .table import Table
    from .columns import ColumnProxy
    from .options import QueryOption

# Import from Rust engine when available
try:
    from data_bridge.data_bridge import postgres as _engine
except ImportError:
    _engine = None


T = TypeVar("T", bound="Table")


class WindowSpec:
    """Window specification for PARTITION BY and ORDER BY."""

    def __init__(self):
        self._partition_by: list[str] = []
        self._order_by: list[tuple[str, str]] = []

    def partition_by(self, *columns: Union[str, "ColumnProxy"]) -> "WindowSpec":
        """Add PARTITION BY columns."""
        spec = WindowSpec()
        spec._partition_by = self._partition_by.copy()
        spec._order_by = self._order_by.copy()
        for col in columns:
            col_name = col.name if hasattr(col, 'name') else col
            spec._partition_by.append(col_name)
        return spec

    def order_by(self, column: Union[str, "ColumnProxy"], direction: str = "asc") -> "WindowSpec":
        """Add ORDER BY column."""
        spec = WindowSpec()
        spec._partition_by = self._partition_by.copy()
        spec._order_by = self._order_by.copy()
        col_name = column.name if hasattr(column, 'name') else column
        spec._order_by.append((col_name, direction))
        return spec


class QueryBuilder(Generic[T]):
    """
    Chainable query builder for PostgreSQL operations.

    Provides a fluent API for building and executing queries.
    All terminal operations (to_list, first, count, exists, aggregate) are async.

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

        >>> # Aggregate queries
        >>> results = await Order.find(Order.status == "completed") \\
        ...     .sum(Order.amount, "total_amount") \\
        ...     .avg(Order.amount, "avg_amount") \\
        ...     .count_agg("order_count") \\
        ...     .group_by("user_id") \\
        ...     .order_by("-total_amount") \\
        ...     .limit(10) \\
        ...     .aggregate()

        >>> # Aggregate with HAVING clause
        >>> results = await Order.find() \\
        ...     .sum(Order.amount, "total") \\
        ...     .group_by("user_id") \\
        ...     .having_sum(Order.amount, ">", 1000) \\
        ...     .aggregate()
    """

    def __init__(
        self,
        model: Type[T],
        filters: tuple,
        _order_by: Optional[List[tuple]] = None,
        _offset: int = 0,
        _limit: int = 0,
        _select: Optional[List[str]] = None,
        _aggregates: Optional[List[tuple]] = None,
        _group_by: Optional[List[str]] = None,
        _having: Optional[List[tuple]] = None,
        _distinct: Optional[bool] = None,
        _distinct_on: Optional[List[str]] = None,
        _ctes: Optional[List[tuple]] = None,
        _subqueries: Optional[List[tuple]] = None,
        _windows: Optional[List[tuple]] = None,
        _set_operations: Optional[List[tuple]] = None,
        _returning: Optional[List[str]] = None,
        _options: Optional[List['QueryOption']] = None,
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
            _aggregates: Aggregate functions [(func_type, column, alias), ...]
            _group_by: GROUP BY columns
            _having: HAVING conditions [(func_type, column, operator, value), ...]
            _distinct: Enable DISTINCT (unique rows only)
            _distinct_on: DISTINCT ON columns (PostgreSQL-specific)
            _ctes: Common Table Expressions [(name, sql, params), ...]
            _subqueries: Subquery conditions [(type, field, sql, params), ...]
            _windows: Window functions [(func_type, column, offset, default, partition_by, order_by, alias), ...]
            _set_operations: Set operations [(op_type, sql, params), ...]
            _returning: Columns to return from UPDATE/DELETE operations
            _options: Query options for eager loading relationships
        """
        self._model = model
        self._filters = filters
        self._order_by_spec = _order_by or []
        self._offset_val = _offset
        self._limit_val = _limit
        self._select_cols = _select
        self._aggregates_spec = _aggregates or []
        self._group_by_cols = _group_by or []
        self._having_conditions = _having or []
        self._distinct: bool = _distinct if _distinct is not None else False
        self._distinct_on_cols: list[str] = _distinct_on or []
        self._ctes: list[tuple[str, str, list[Any]]] = _ctes or []  # (name, sql, params)
        self._subqueries: list[tuple[str, str | None, str, list[Any]]] = _subqueries or []  # (type, field, sql, params)
        self._windows: list[tuple[str, str | None, int | None, Any, list[str], list[tuple[str, str]], str]] = _windows or []  # (func_type, column, offset, default, partition_by, order_by, alias)
        self._set_operations: list[tuple[str, str, list[Any]]] = _set_operations or []  # (op_type, sql, params)
        self._returning: list[str] = _returning or []  # Columns to return from UPDATE/DELETE
        self._json_conditions: list[tuple[str, str, Any]] = []  # (operator_type, column, value)
        self._options: list['QueryOption'] = _options or []  # Query options for eager loading

    def _clone(self, **kwargs: Any) -> "QueryBuilder[T]":
        """Create a copy of this builder with updated values."""
        cloned = QueryBuilder(
            model=kwargs.get("model", self._model),
            filters=kwargs.get("filters", self._filters),
            _order_by=kwargs.get("_order_by", self._order_by_spec.copy()),
            _offset=kwargs.get("_offset", self._offset_val),
            _limit=kwargs.get("_limit", self._limit_val),
            _select=kwargs.get("_select", self._select_cols),
            _aggregates=kwargs.get("_aggregates", self._aggregates_spec.copy()),
            _group_by=kwargs.get("_group_by", self._group_by_cols.copy()),
            _having=kwargs.get("_having", self._having_conditions.copy()),
            _distinct=kwargs.get("_distinct", self._distinct),
            _distinct_on=kwargs.get("_distinct_on", self._distinct_on_cols.copy()),
            _ctes=kwargs.get("_ctes", self._ctes.copy()),
            _subqueries=kwargs.get("_subqueries", self._subqueries.copy()),
            _windows=kwargs.get("_windows", self._windows.copy()),
            _set_operations=kwargs.get("_set_operations", self._set_operations.copy()),
            _returning=kwargs.get("_returning", self._returning.copy()),
            _options=kwargs.get("_options", self._options.copy()),
        )
        cloned._json_conditions = kwargs.get("_json_conditions", self._json_conditions.copy())
        return cloned

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

    def distinct(self) -> "QueryBuilder[T]":
        """
        Return only distinct (unique) rows.

        Returns:
            A new QueryBuilder with DISTINCT enabled.

        Example:
            >>> # Get unique email addresses
            >>> emails = await User.find().select("email").distinct().to_list()
        """
        new_qb = self._clone()
        new_qb._distinct = True
        return new_qb

    def distinct_on(self, *columns: Union[str, "ColumnProxy"]) -> "QueryBuilder[T]":
        """
        Return first row for each unique combination of columns (PostgreSQL-specific).

        Note: ORDER BY should typically start with the DISTINCT ON columns.

        Args:
            *columns: Column names or ColumnProxy objects to group uniqueness by.

        Returns:
            A new QueryBuilder with DISTINCT ON enabled.

        Example:
            >>> # Get latest order per user
            >>> orders = await Order.find() \\
            ...     .distinct_on(Order.user_id) \\
            ...     .order_by("-created_at") \\
            ...     .to_list()
        """
        from .columns import ColumnProxy

        new_qb = self._clone()
        for col in columns:
            col_name = col.name if isinstance(col, ColumnProxy) else col
            new_qb._distinct_on_cols.append(col_name)
        return new_qb

    def count_agg(self, alias: Optional[str] = None) -> "QueryBuilder[T]":
        """
        Add COUNT(*) aggregate to the query.

        Args:
            alias: Optional alias for the aggregate result

        Returns:
            New QueryBuilder with COUNT(*) aggregate applied

        Example:
            >>> # Count all rows grouped by status
            >>> results = await User.find() \\
            ...     .count_agg("total") \\
            ...     .group_by("status") \\
            ...     .aggregate()
        """
        new_aggregates = self._aggregates_spec.copy()
        new_aggregates.append(("count", None, alias))
        return self._clone(_aggregates=new_aggregates)

    def count_column(self, column: Union[ColumnProxy, str], alias: Optional[str] = None) -> "QueryBuilder[T]":
        """
        Add COUNT(column) aggregate to the query.

        Args:
            column: Column to count (non-NULL values)
            alias: Optional alias for the aggregate result

        Returns:
            New QueryBuilder with COUNT(column) aggregate applied

        Example:
            >>> # Count non-null email addresses grouped by status
            >>> results = await User.find() \\
            ...     .count_column(User.email, "email_count") \\
            ...     .group_by("status") \\
            ...     .aggregate()
        """
        from .columns import ColumnProxy

        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_aggregates = self._aggregates_spec.copy()
        new_aggregates.append(("count_column", col_name, alias))
        return self._clone(_aggregates=new_aggregates)

    def count_distinct(self, column: Union[ColumnProxy, str], alias: Optional[str] = None) -> "QueryBuilder[T]":
        """
        Add COUNT(DISTINCT column) aggregate to the query.

        Args:
            column: Column to count distinct values
            alias: Optional alias for the aggregate result

        Returns:
            New QueryBuilder with COUNT(DISTINCT column) aggregate applied

        Example:
            >>> # Count unique user IDs per status
            >>> results = await Order.find() \\
            ...     .count_distinct(Order.user_id, "unique_users") \\
            ...     .group_by("status") \\
            ...     .aggregate()
        """
        from .columns import ColumnProxy

        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_aggregates = self._aggregates_spec.copy()
        new_aggregates.append(("count_distinct", col_name, alias))
        return self._clone(_aggregates=new_aggregates)

    def sum(self, column: Union[ColumnProxy, str], alias: Optional[str] = None) -> "QueryBuilder[T]":
        """
        Add SUM(column) aggregate to the query.

        Args:
            column: Column to sum
            alias: Optional alias for the aggregate result

        Returns:
            New QueryBuilder with SUM(column) aggregate applied

        Example:
            >>> # Calculate total amount per user
            >>> results = await Order.find() \\
            ...     .sum(Order.amount, "total_amount") \\
            ...     .group_by("user_id") \\
            ...     .aggregate()
        """
        from .columns import ColumnProxy

        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_aggregates = self._aggregates_spec.copy()
        new_aggregates.append(("sum", col_name, alias))
        return self._clone(_aggregates=new_aggregates)

    def avg(self, column: Union[ColumnProxy, str], alias: Optional[str] = None) -> "QueryBuilder[T]":
        """
        Add AVG(column) aggregate to the query.

        Args:
            column: Column to average
            alias: Optional alias for the aggregate result

        Returns:
            New QueryBuilder with AVG(column) aggregate applied

        Example:
            >>> # Calculate average order amount per user
            >>> results = await Order.find() \\
            ...     .avg(Order.amount, "avg_amount") \\
            ...     .group_by("user_id") \\
            ...     .aggregate()
        """
        from .columns import ColumnProxy

        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_aggregates = self._aggregates_spec.copy()
        new_aggregates.append(("avg", col_name, alias))
        return self._clone(_aggregates=new_aggregates)

    def min(self, column: Union[ColumnProxy, str], alias: Optional[str] = None) -> "QueryBuilder[T]":
        """
        Add MIN(column) aggregate to the query.

        Args:
            column: Column to find minimum value
            alias: Optional alias for the aggregate result

        Returns:
            New QueryBuilder with MIN(column) aggregate applied

        Example:
            >>> # Find minimum order amount per user
            >>> results = await Order.find() \\
            ...     .min(Order.amount, "min_amount") \\
            ...     .group_by("user_id") \\
            ...     .aggregate()
        """
        from .columns import ColumnProxy

        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_aggregates = self._aggregates_spec.copy()
        new_aggregates.append(("min", col_name, alias))
        return self._clone(_aggregates=new_aggregates)

    def max(self, column: Union[ColumnProxy, str], alias: Optional[str] = None) -> "QueryBuilder[T]":
        """
        Add MAX(column) aggregate to the query.

        Args:
            column: Column to find maximum value
            alias: Optional alias for the aggregate result

        Returns:
            New QueryBuilder with MAX(column) aggregate applied

        Example:
            >>> # Find maximum order amount per user
            >>> results = await Order.find() \\
            ...     .max(Order.amount, "max_amount") \\
            ...     .group_by("user_id") \\
            ...     .aggregate()
        """
        from .columns import ColumnProxy

        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_aggregates = self._aggregates_spec.copy()
        new_aggregates.append(("max", col_name, alias))
        return self._clone(_aggregates=new_aggregates)

    def group_by(self, *columns: Union[ColumnProxy, str]) -> "QueryBuilder[T]":
        """
        Add GROUP BY columns to the query.

        Args:
            *columns: Column proxies or column names to group by

        Returns:
            New QueryBuilder with GROUP BY applied

        Example:
            >>> # Group by single column
            >>> results = await User.find() \\
            ...     .count_agg("total") \\
            ...     .group_by(User.status) \\
            ...     .aggregate()
            >>>
            >>> # Group by multiple columns
            >>> results = await Order.find() \\
            ...     .sum(Order.amount, "total") \\
            ...     .group_by("user_id", "status") \\
            ...     .aggregate()
        """
        from .columns import ColumnProxy

        column_names = []
        for col in columns:
            if isinstance(col, str):
                column_names.append(col)
            elif isinstance(col, ColumnProxy):
                column_names.append(col.name)
            else:
                raise TypeError(f"Invalid group_by column type: {type(col)}")

        new_group_by = self._group_by_cols.copy()
        new_group_by.extend(column_names)
        return self._clone(_group_by=new_group_by)

    def having(
        self,
        aggregate: str,
        column: Union[str, ColumnProxy, None],
        operator: str,
        value: Any,
    ) -> "QueryBuilder[T]":
        """
        Add a HAVING condition to filter aggregate results.

        Args:
            aggregate: The aggregate function type ("count", "sum", "avg", "min", "max")
            column: The column to aggregate (None for COUNT(*))
            operator: The comparison operator ("=", ">", ">=", "<", "<=", "!=")
            value: The value to compare against

        Returns:
            A new QueryBuilder with the HAVING condition added.

        Example:
            >>> # Filter groups where SUM(amount) > 1000
            >>> results = await Order.find() \\
            ...     .sum("amount", "total") \\
            ...     .group_by("user_id") \\
            ...     .having("sum", "amount", ">", 1000) \\
            ...     .aggregate()
            >>>
            >>> # Can also use ColumnProxy
            >>> results = await Order.find() \\
            ...     .sum(Order.amount, "total") \\
            ...     .group_by("user_id") \\
            ...     .having("sum", Order.amount, ">", 1000) \\
            ...     .aggregate()
        """
        from .columns import ColumnProxy

        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_having = self._having_conditions.copy()
        new_having.append((aggregate, col_name, operator, value))
        return self._clone(_having=new_having)

    def having_count(self, operator: str, value: int) -> "QueryBuilder[T]":
        """
        Add HAVING COUNT(*) condition.

        Args:
            operator: The comparison operator ("=", ">", ">=", "<", "<=", "!=")
            value: The value to compare against

        Returns:
            A new QueryBuilder with the HAVING COUNT(*) condition added.

        Example:
            >>> # Groups with more than 5 orders
            >>> results = await Order.find() \\
            ...     .count_agg("order_count") \\
            ...     .group_by("user_id") \\
            ...     .having_count(">", 5) \\
            ...     .aggregate()
        """
        return self.having("count", None, operator, value)

    def having_sum(self, column: Union[str, ColumnProxy], operator: str, value: Union[float, int]) -> "QueryBuilder[T]":
        """
        Add HAVING SUM(column) condition.

        Args:
            column: The column to sum
            operator: The comparison operator ("=", ">", ">=", "<", "<=", "!=")
            value: The value to compare against

        Returns:
            A new QueryBuilder with the HAVING SUM condition added.

        Example:
            >>> # Groups where total amount > 1000
            >>> results = await Order.find() \\
            ...     .sum(Order.amount, "total") \\
            ...     .group_by("user_id") \\
            ...     .having_sum(Order.amount, ">", 1000) \\
            ...     .aggregate()
        """
        return self.having("sum", column, operator, value)

    def having_avg(self, column: Union[str, ColumnProxy], operator: str, value: Union[float, int]) -> "QueryBuilder[T]":
        """
        Add HAVING AVG(column) condition.

        Args:
            column: The column to average
            operator: The comparison operator ("=", ">", ">=", "<", "<=", "!=")
            value: The value to compare against

        Returns:
            A new QueryBuilder with the HAVING AVG condition added.

        Example:
            >>> # Groups where average amount >= 100
            >>> results = await Order.find() \\
            ...     .avg(Order.amount, "average") \\
            ...     .group_by("user_id") \\
            ...     .having_avg(Order.amount, ">=", 100) \\
            ...     .aggregate()
        """
        return self.having("avg", column, operator, value)

    def having_min(self, column: Union[str, ColumnProxy], operator: str, value: Any) -> "QueryBuilder[T]":
        """
        Add HAVING MIN(column) condition.

        Args:
            column: The column to find minimum value
            operator: The comparison operator ("=", ">", ">=", "<", "<=", "!=")
            value: The value to compare against

        Returns:
            A new QueryBuilder with the HAVING MIN condition added.

        Example:
            >>> # Groups where minimum order amount > 50
            >>> results = await Order.find() \\
            ...     .min(Order.amount, "min_order") \\
            ...     .group_by("user_id") \\
            ...     .having_min(Order.amount, ">", 50) \\
            ...     .aggregate()
        """
        return self.having("min", column, operator, value)

    def having_max(self, column: Union[str, ColumnProxy], operator: str, value: Any) -> "QueryBuilder[T]":
        """
        Add HAVING MAX(column) condition.

        Args:
            column: The column to find maximum value
            operator: The comparison operator ("=", ">", ">=", "<", "<=", "!=")
            value: The value to compare against

        Returns:
            A new QueryBuilder with the HAVING MAX condition added.

        Example:
            >>> # Groups where maximum order amount < 10000
            >>> results = await Order.find() \\
            ...     .max(Order.amount, "max_order") \\
            ...     .group_by("user_id") \\
            ...     .having_max(Order.amount, "<", 10000) \\
            ...     .aggregate()
        """
        return self.having("max", column, operator, value)

    def window(
        self,
        func_type: str,
        alias: str,
        column: Union[str, ColumnProxy, None] = None,
        spec: WindowSpec | None = None,
        offset: int | None = None,
        default: Any = None,
    ) -> "QueryBuilder[T]":
        """Add a window function to the query.

        Args:
            func_type: Window function type ("row_number", "rank", "sum", "lag", etc.)
            alias: Alias for the result column.
            column: Column name for functions that need it.
            spec: WindowSpec with PARTITION BY and ORDER BY.
            offset: Offset for LAG/LEAD functions.
            default: Default value for LAG/LEAD functions.

        Returns:
            A new QueryBuilder with the window function added.

        Example:
            >>> results = await Order.find() \\
            ...     .window("row_number", "rank",
            ...             spec=WindowSpec().order_by(Order.amount, "desc")) \\
            ...     .aggregate()
        """
        new_qb = self._clone()
        col_name = column.name if hasattr(column, 'name') else column
        partition_by = spec._partition_by if spec else []
        order_by = spec._order_by if spec else []
        new_qb._windows.append((func_type, col_name, offset, default, partition_by, order_by, alias))
        return new_qb

    def row_number(self, alias: str, spec: WindowSpec | None = None) -> "QueryBuilder[T]":
        """Add ROW_NUMBER() window function."""
        return self.window("row_number", alias, spec=spec)

    def rank(self, alias: str, spec: WindowSpec | None = None) -> "QueryBuilder[T]":
        """Add RANK() window function."""
        return self.window("rank", alias, spec=spec)

    def dense_rank(self, alias: str, spec: WindowSpec | None = None) -> "QueryBuilder[T]":
        """Add DENSE_RANK() window function."""
        return self.window("dense_rank", alias, spec=spec)

    def window_sum(self, column: Union[str, ColumnProxy], alias: str, spec: WindowSpec | None = None) -> "QueryBuilder[T]":
        """Add SUM() as window function."""
        return self.window("sum", alias, column=column, spec=spec)

    def window_avg(self, column: Union[str, ColumnProxy], alias: str, spec: WindowSpec | None = None) -> "QueryBuilder[T]":
        """Add AVG() as window function."""
        return self.window("avg", alias, column=column, spec=spec)

    def lag(
        self,
        column: Union[str, ColumnProxy],
        alias: str,
        offset: int = 1,
        default: Any = None,
        spec: WindowSpec | None = None,
    ) -> "QueryBuilder[T]":
        """Add LAG() window function to access previous row."""
        return self.window("lag", alias, column=column, spec=spec, offset=offset, default=default)

    def lead(
        self,
        column: Union[str, ColumnProxy],
        alias: str,
        offset: int = 1,
        default: Any = None,
        spec: WindowSpec | None = None,
    ) -> "QueryBuilder[T]":
        """Add LEAD() window function to access next row."""
        return self.window("lead", alias, column=column, spec=spec, offset=offset, default=default)

    def with_cte(self, name: str, query: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """
        Add a Common Table Expression (CTE) to the query.

        CTEs are defined in the WITH clause and can be referenced in the main query.

        Args:
            name: The name for this CTE (used to reference it in the main query).
            query: A QueryBuilder that defines the CTE's query.

        Returns:
            A new QueryBuilder with the CTE added.

        Example:
            >>> # Define a CTE for high-value orders
            >>> high_value = Order.find(Order.total > 1000)
            >>>
            >>> # Use it in the main query (reference by name in raw SQL or via from_cte)
            >>> results = await QueryBuilder.from_cte("high_value_orders", high_value) \\
            ...     .where(total > 5000) \\
            ...     .to_list()
        """
        new_qb = self._clone()
        # Build the CTE query's SQL
        cte_sql, cte_params = query._build_sql()
        new_qb._ctes.append((name, cte_sql, cte_params))
        return new_qb

    def with_cte_raw(self, name: str, sql: str, params: Optional[List[Any]] = None) -> "QueryBuilder[T]":
        """
        Add a raw SQL CTE to the query.

        Args:
            name: The name for this CTE.
            sql: The raw SQL query for the CTE.
            params: Optional parameters for the SQL query.

        Returns:
            A new QueryBuilder with the CTE added.

        Example:
            >>> results = await Order.find() \\
            ...     .with_cte_raw(
            ...         "monthly_totals",
            ...         "SELECT user_id, SUM(amount) as total FROM orders GROUP BY user_id"
            ...     ) \\
            ...     .to_list()
        """
        new_qb = self._clone()
        new_qb._ctes.append((name, sql, params or []))
        return new_qb

    def where_in_subquery(
        self,
        column: Union[str, "ColumnProxy"],
        subquery: "QueryBuilder[Any]",
    ) -> "QueryBuilder[T]":
        """Add a WHERE column IN (subquery) condition.

        Args:
            column: The column to check.
            subquery: A QueryBuilder defining the subquery.

        Returns:
            A new QueryBuilder with the condition added.

        Example:
            >>> # Find users who have orders
            >>> order_users = Order.find().select(Order.user_id)
            >>> users = await User.find() \\
            ...     .where_in_subquery(User.id, order_users) \\
            ...     .to_list()
        """
        from .columns import ColumnProxy

        new_qb = self._clone()
        col_name = column.name if isinstance(column, ColumnProxy) else column
        sql, params = subquery._build_sql()
        new_qb._subqueries.append(("in", col_name, sql, params))
        return new_qb

    def where_not_in_subquery(
        self,
        column: Union[str, "ColumnProxy"],
        subquery: "QueryBuilder[Any]",
    ) -> "QueryBuilder[T]":
        """Add a WHERE column NOT IN (subquery) condition.

        Args:
            column: The column to check.
            subquery: A QueryBuilder defining the subquery.

        Returns:
            A new QueryBuilder with the condition added.

        Example:
            >>> # Find users who have no orders
            >>> order_users = Order.find().select(Order.user_id)
            >>> users = await User.find() \\
            ...     .where_not_in_subquery(User.id, order_users) \\
            ...     .to_list()
        """
        from .columns import ColumnProxy

        new_qb = self._clone()
        col_name = column.name if isinstance(column, ColumnProxy) else column
        sql, params = subquery._build_sql()
        new_qb._subqueries.append(("not_in", col_name, sql, params))
        return new_qb

    def where_exists(self, subquery: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """Add a WHERE EXISTS (subquery) condition.

        Args:
            subquery: A QueryBuilder defining the subquery.

        Returns:
            A new QueryBuilder with the condition added.

        Example:
            >>> # Find users who have at least one order
            >>> has_orders = Order.find()  # Will be correlated
            >>> users = await User.find() \\
            ...     .where_exists(has_orders) \\
            ...     .to_list()
        """
        new_qb = self._clone()
        sql, params = subquery._build_sql()
        new_qb._subqueries.append(("exists", None, sql, params))
        return new_qb

    def where_not_exists(self, subquery: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """Add a WHERE NOT EXISTS (subquery) condition.

        Args:
            subquery: A QueryBuilder defining the subquery.

        Returns:
            A new QueryBuilder with the condition added.

        Example:
            >>> # Find users who have no orders
            >>> has_orders = Order.find()
            >>> users = await User.find() \\
            ...     .where_not_exists(has_orders) \\
            ...     .to_list()
        """
        new_qb = self._clone()
        sql, params = subquery._build_sql()
        new_qb._subqueries.append(("not_exists", None, sql, params))
        return new_qb

    def where_in_raw(
        self,
        column: Union[str, "ColumnProxy"],
        sql: str,
        params: Optional[List[Any]] = None,
    ) -> "QueryBuilder[T]":
        """Add a WHERE column IN (raw SQL) condition.

        Use this for complex subqueries that can't be built with QueryBuilder.

        Args:
            column: The column to check.
            sql: Raw SQL for the subquery.
            params: Parameters for the SQL.

        Returns:
            A new QueryBuilder with the condition added.

        Example:
            >>> users = await User.find() \\
            ...     .where_in_raw(User.id, "SELECT user_id FROM orders WHERE total > $1", [1000]) \\
            ...     .to_list()
        """
        from .columns import ColumnProxy

        new_qb = self._clone()
        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_qb._subqueries.append(("in", col_name, sql, params or []))
        return new_qb

    def where_exists_raw(self, sql: str, params: Optional[List[Any]] = None) -> "QueryBuilder[T]":
        """Add a WHERE EXISTS (raw SQL) condition.

        Use this for complex EXISTS queries that can't be built with QueryBuilder.

        Args:
            sql: Raw SQL for the EXISTS subquery.
            params: Parameters for the SQL.

        Returns:
            A new QueryBuilder with the condition added.

        Example:
            >>> users = await User.find() \\
            ...     .where_exists_raw("SELECT 1 FROM orders WHERE user_id = users.id AND total > $1", [1000]) \\
            ...     .to_list()
        """
        new_qb = self._clone()
        new_qb._subqueries.append(("exists", None, sql, params or []))
        return new_qb

    @classmethod
    def from_cte(cls, cte_name: str, cte_query: "QueryBuilder[Any]", model: Optional[Type[T]] = None) -> "QueryBuilder[T]":
        """
        Create a QueryBuilder that queries from a CTE.

        This is a convenience method that creates a query targeting the CTE name
        and includes the CTE definition.

        Args:
            cte_name: The name to give the CTE.
            cte_query: The QueryBuilder defining the CTE.
            model: Optional model class for result deserialization.

        Returns:
            A new QueryBuilder that will query from the CTE.

        Example:
            >>> high_value = Order.find(Order.total > 1000)
            >>> results = await QueryBuilder.from_cte("high_value", high_value, Order) \\
            ...     .order_by("-total") \\
            ...     .limit(10) \\
            ...     .to_list()
        """
        # Build the CTE query's SQL
        cte_sql, cte_params = cte_query._build_sql()

        # Create a new QueryBuilder
        # We'll use the CTE query's model if no model is provided
        result_model = model if model is not None else cte_query._model

        new_qb = cls(
            model=result_model,
            filters=(),  # No filters initially
            _ctes=[(cte_name, cte_sql, cte_params)],
        )

        # Override the table name to use the CTE
        # This is a bit of a hack, but we need to query from the CTE name
        # Store the original model for result construction
        new_qb._cte_table_name = cte_name

        return new_qb

    def union(self, other: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """
        Combine with another query using UNION (removes duplicates).

        Args:
            other: Another QueryBuilder to combine with

        Returns:
            New QueryBuilder with UNION operation

        Example:
            >>> # Find active and archived users
            >>> active = User.find(User.status == "active")
            >>> archived = User.find(User.status == "archived")
            >>> all_users = await active.union(archived).aggregate()
        """
        new_qb = self._clone()
        sql, params = other._build_sql()
        new_qb._set_operations.append(("union", sql, params))
        return new_qb

    def union_all(self, other: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """
        Combine with UNION ALL (keeps duplicates).

        Args:
            other: Another QueryBuilder to combine with

        Returns:
            New QueryBuilder with UNION ALL operation

        Example:
            >>> active = User.find(User.status == "active")
            >>> pending = User.find(User.status == "pending")
            >>> all_users = await active.union_all(pending).aggregate()
        """
        new_qb = self._clone()
        sql, params = other._build_sql()
        new_qb._set_operations.append(("union_all", sql, params))
        return new_qb

    def intersect(self, other: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """
        Return only rows present in both queries (removes duplicates).

        Args:
            other: Another QueryBuilder to intersect with

        Returns:
            New QueryBuilder with INTERSECT operation

        Example:
            >>> # Find users who are both admins and active
            >>> admins = User.find(User.role == "admin")
            >>> active = User.find(User.status == "active")
            >>> active_admins = await admins.intersect(active).aggregate()
        """
        new_qb = self._clone()
        sql, params = other._build_sql()
        new_qb._set_operations.append(("intersect", sql, params))
        return new_qb

    def intersect_all(self, other: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """
        Return only rows present in both queries (keeps duplicates).

        Args:
            other: Another QueryBuilder to intersect with

        Returns:
            New QueryBuilder with INTERSECT ALL operation
        """
        new_qb = self._clone()
        sql, params = other._build_sql()
        new_qb._set_operations.append(("intersect_all", sql, params))
        return new_qb

    def except_(self, other: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """
        Return rows in this query but not in the other (removes duplicates).

        Note: Named except_ to avoid conflict with Python's except keyword.

        Args:
            other: Another QueryBuilder to subtract

        Returns:
            New QueryBuilder with EXCEPT operation

        Example:
            >>> # Find users who are admins but not suspended
            >>> admins = User.find(User.role == "admin")
            >>> suspended = User.find(User.status == "suspended")
            >>> active_admins = await admins.except_(suspended).aggregate()
        """
        new_qb = self._clone()
        sql, params = other._build_sql()
        new_qb._set_operations.append(("except", sql, params))
        return new_qb

    def except_all(self, other: "QueryBuilder[Any]") -> "QueryBuilder[T]":
        """
        Return rows in this query but not in the other (keeps duplicates).

        Args:
            other: Another QueryBuilder to subtract

        Returns:
            New QueryBuilder with EXCEPT ALL operation
        """
        new_qb = self._clone()
        sql, params = other._build_sql()
        new_qb._set_operations.append(("except_all", sql, params))
        return new_qb

    def where_json_contains(
        self,
        column: Union[str, "ColumnProxy"],
        json_value: Union[str, dict],
    ) -> "QueryBuilder[T]":
        """Filter where JSONB column contains the given JSON.

        Args:
            column: The JSONB column to check.
            json_value: JSON string or dict to check for containment.

        Returns:
            New QueryBuilder with the condition added.

        Example:
            >>> users = await User.find() \\
            ...     .where_json_contains(User.metadata, {"role": "admin"}) \\
            ...     .to_list()
        """
        import json
        from .columns import ColumnProxy

        new_qb = self._clone()
        col_name = column.name if isinstance(column, ColumnProxy) else column
        json_str = json.dumps(json_value) if isinstance(json_value, dict) else json_value
        new_qb._json_conditions.append(("json_contains", col_name, json_str))
        return new_qb

    def where_json_contained_by(
        self,
        column: Union[str, "ColumnProxy"],
        json_value: Union[str, dict],
    ) -> "QueryBuilder[T]":
        """Filter where JSONB column is contained by the given JSON.

        Args:
            column: The JSONB column to check.
            json_value: JSON string or dict to check containment.

        Returns:
            New QueryBuilder with the condition added.

        Example:
            >>> users = await User.find() \\
            ...     .where_json_contained_by(User.metadata, {"premium": True}) \\
            ...     .to_list()
        """
        import json
        from .columns import ColumnProxy

        new_qb = self._clone()
        col_name = column.name if isinstance(column, ColumnProxy) else column
        json_str = json.dumps(json_value) if isinstance(json_value, dict) else json_value
        new_qb._json_conditions.append(("json_contained_by", col_name, json_str))
        return new_qb

    def where_json_key_exists(
        self,
        column: Union[str, "ColumnProxy"],
        key: str,
    ) -> "QueryBuilder[T]":
        """Filter where JSONB column has the specified key.

        Args:
            column: The JSONB column to check.
            key: The key to check for existence.

        Returns:
            New QueryBuilder with the condition added.

        Example:
            >>> users = await User.find() \\
            ...     .where_json_key_exists(User.metadata, "email") \\
            ...     .to_list()
        """
        from .columns import ColumnProxy

        new_qb = self._clone()
        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_qb._json_conditions.append(("json_key_exists", col_name, key))
        return new_qb

    def where_json_any_key_exists(
        self,
        column: Union[str, "ColumnProxy"],
        keys: list[str],
    ) -> "QueryBuilder[T]":
        """Filter where JSONB column has any of the specified keys.

        Args:
            column: The JSONB column to check.
            keys: List of keys to check (matches if ANY exist).

        Returns:
            New QueryBuilder with the condition added.

        Example:
            >>> users = await User.find() \\
            ...     .where_json_any_key_exists(User.metadata, ["email", "phone"]) \\
            ...     .to_list()
        """
        from .columns import ColumnProxy

        new_qb = self._clone()
        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_qb._json_conditions.append(("json_any_key_exists", col_name, keys))
        return new_qb

    def where_json_all_keys_exist(
        self,
        column: Union[str, "ColumnProxy"],
        keys: list[str],
    ) -> "QueryBuilder[T]":
        """Filter where JSONB column has all of the specified keys.

        Args:
            column: The JSONB column to check.
            keys: List of keys to check (matches only if ALL exist).

        Returns:
            New QueryBuilder with the condition added.

        Example:
            >>> users = await User.find() \\
            ...     .where_json_all_keys_exist(User.metadata, ["name", "email"]) \\
            ...     .to_list()
        """
        from .columns import ColumnProxy

        new_qb = self._clone()
        col_name = column.name if isinstance(column, ColumnProxy) else column
        new_qb._json_conditions.append(("json_all_keys_exist", col_name, keys))
        return new_qb

    def returning(self, *columns: Union[str, "ColumnProxy"]) -> "QueryBuilder[T]":
        """Specify columns to return from UPDATE/DELETE operations.

        Note: This is reserved for future use when UPDATE/DELETE methods
        are added to QueryBuilder. Currently, use Table.update_many() and
        Table.delete_many() with the returning parameter instead.

        Args:
            *columns: Column names or ColumnProxy objects to return.

        Returns:
            A new QueryBuilder with RETURNING clause specification.

        Example:
            >>> # Use with Table.update_many() / Table.delete_many()
            >>> results = await User.update_many(
            ...     {"status": "inactive"},
            ...     User.id == 1,
            ...     returning=["id", "name", "status"]
            ... )
        """
        new_qb = self._clone()
        for col in columns:
            col_name = col.name if hasattr(col, 'name') else col
            new_qb._returning.append(col_name)
        return new_qb

    def returning_all(self) -> "QueryBuilder[T]":
        """Return all columns from UPDATE/DELETE operations.

        Note: This is reserved for future use when UPDATE/DELETE methods
        are added to QueryBuilder. Currently, use Table.update_many() and
        Table.delete_many() with the returning parameter instead.

        Example:
            >>> # Use with Table.delete_many()
            >>> results = await User.delete_many(
            ...     User.id == 1,
            ...     returning=["*"]
            ... )
        """
        new_qb = self._clone()
        new_qb._returning.append("*")
        return new_qb

    def jsonb_contains(self, column: str, value: dict) -> 'QueryBuilder[T]':
        """Filter by JSONB contains (@> operator).

        Args:
            column: JSONB column name
            value: Dict to check containment

        Returns:
            Self for chaining

        Example:
            >>> users = await User.find().jsonb_contains("metadata", {"role": "admin"}).to_list()
        """
        import json
        json_str = json.dumps(value).replace("'", "''")
        return self.where_json_contains(column, json_str)

    def jsonb_contained_by(self, column: str, value: dict) -> 'QueryBuilder[T]':
        """Filter by JSONB contained by (<@ operator).

        Args:
            column: JSONB column name
            value: Dict to check containment

        Returns:
            Self for chaining
        """
        import json
        json_str = json.dumps(value).replace("'", "''")
        return self.where_json_contained_by(column, json_str)

    def jsonb_has_key(self, column: str, key: str) -> 'QueryBuilder[T]':
        """Filter by JSONB has key (? operator).

        Args:
            column: JSONB column name
            key: Key to check existence

        Returns:
            Self for chaining

        Example:
            >>> users = await User.find().jsonb_has_key("settings", "theme").to_list()
        """
        return self.where_json_key_exists(column, key)

    def jsonb_has_any_key(self, column: str, keys: list) -> 'QueryBuilder[T]':
        """Filter by JSONB has any of the keys (?| operator).

        Args:
            column: JSONB column name
            keys: List of keys to check

        Returns:
            Self for chaining
        """
        return self.where_json_any_key_exists(column, keys)

    def jsonb_has_all_keys(self, column: str, keys: list) -> 'QueryBuilder[T]':
        """Filter by JSONB has all keys (?& operator).

        Args:
            column: JSONB column name
            keys: List of keys that must all exist

        Returns:
            Self for chaining
        """
        return self.where_json_all_keys_exist(column, keys)

    def options(self, *options: 'QueryOption') -> "QueryBuilder[T]":
        """Specify eager loading options for relationships.

        This allows you to control how relationships are loaded,
        preventing N+1 query problems by batch loading related objects.

        Args:
            *options: QueryOption instances (selectinload, joinedload, noload, etc.)

        Returns:
            New QueryBuilder with eager loading options applied

        Example:
            >>> from data_bridge.postgres import selectinload
            >>>
            >>> # Load posts with their authors in 2 queries (instead of N+1)
            >>> posts = await Post.find().options(selectinload("author")).to_list()
            >>>
            >>> # All authors already loaded
            >>> for post in posts:
            ...     author = await post.author  # No additional query
            ...     print(f"{post.title} by {author.name}")
            >>>
            >>> # Load multiple relationships
            >>> posts = await Post.find().options(
            ...     selectinload("author"),
            ...     selectinload("comments")
            ... ).to_list()
        """
        new_options = self._options.copy()
        new_options.extend(options)
        return self._clone(_options=new_options)

    def _build_sql(self) -> tuple[str, List[Any]]:
        """
        Build SQL and params from current QueryBuilder state (for CTE usage).

        This is a simplified SQL builder for CTE definitions.
        For complex cases, the raw SQL approach should be used.

        Returns:
            Tuple of (sql_string, parameters_list)
        """
        parts = ["SELECT"]
        params: List[Any] = []

        # DISTINCT clause
        if self._distinct_on_cols:
            distinct_cols = ", ".join(f'"{c}"' for c in self._distinct_on_cols)
            parts.append(f"DISTINCT ON ({distinct_cols})")
        elif self._distinct:
            parts.append("DISTINCT")

        # Columns
        if self._select_cols:
            parts.append(", ".join(f'"{c}"' for c in self._select_cols))
        else:
            parts.append("*")

        # FROM clause
        table_name = self._model.__table_name__()
        parts.append(f'FROM "{table_name}"')

        # WHERE clause
        if self._filters:
            where_clause, where_params = self._build_where_clause()
            if where_clause:
                parts.append(f"WHERE {where_clause}")
                params.extend(where_params)

        # ORDER BY
        if self._order_by_spec:
            order_clauses = [f'"{col}" {direction}' for col, direction in self._order_by_spec]
            parts.append(f"ORDER BY {', '.join(order_clauses)}")

        # LIMIT
        if self._limit_val > 0:
            parts.append(f"LIMIT {self._limit_val}")

        # OFFSET
        if self._offset_val > 0:
            parts.append(f"OFFSET {self._offset_val}")

        return " ".join(parts), params

    async def aggregate(self) -> List[dict]:
        """
        Execute the aggregate query and return results.

        Returns:
            List of dictionaries with aggregate results

        Example:
            >>> # Get total and average amount per user
            >>> results = await Order.find() \\
            ...     .sum(Order.amount, "total") \\
            ...     .avg(Order.amount, "average") \\
            ...     .count_agg("count") \\
            ...     .group_by("user_id") \\
            ...     .aggregate()
            >>> for row in results:
            ...     print(f"User {row['user_id']}: total={row['total']}, avg={row['average']}")
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        if not self._aggregates_spec:
            raise ValueError("No aggregate functions specified. Use count_agg(), sum(), avg(), etc.")

        table_name = self._model.__table_name__()

        # Convert where clause to conditions format expected by query_aggregate
        where_conditions = []
        if self._filters:
            for filter_item in self._filters:
                if isinstance(filter_item, SqlExpr):
                    # Map SQL operators to Rust engine format
                    op_map = {
                        "=": "eq",
                        "!=": "ne",
                        ">": "gt",
                        ">=": "gte",
                        "<": "lt",
                        "<=": "lte",
                        "LIKE": "like",
                        "ILIKE": "ilike",
                        "IN": "in",
                        "IS NULL": "is_null",
                        "IS NOT NULL": "is_not_null",
                    }
                    operator = op_map.get(filter_item.op, filter_item.op.lower())
                    where_conditions.append((filter_item.column, operator, filter_item.value))
                elif isinstance(filter_item, dict):
                    for key, value in filter_item.items():
                        where_conditions.append((key, "eq", value))

        # Add JSON conditions to where_conditions
        for op_type, column, value in self._json_conditions:
            where_conditions.append((column, op_type, value))

        # Fast-path: no tracing overhead if disabled
        if not is_tracing_enabled():
            return await _engine.query_aggregate(
                table=table_name,
                aggregates=self._aggregates_spec,
                group_by=self._group_by_cols if self._group_by_cols else None,
                having=self._having_conditions if self._having_conditions else None,
                where_conditions=where_conditions if where_conditions else None,
                order_by=self._order_by_spec if self._order_by_spec else None,
                limit=self._limit_val if self._limit_val > 0 else None,
                distinct=self._distinct if self._distinct else None,
                distinct_on=self._distinct_on_cols if self._distinct_on_cols else None,
                ctes=self._ctes if self._ctes else None,
                subqueries=self._subqueries if self._subqueries else None,
                windows=self._windows if self._windows else None,
                set_operations=self._set_operations if self._set_operations else None,
            )

        # Create span with aggregate-specific attributes
        aggregates_info = ", ".join(f"{agg[0]}({agg[1] or '*'})" for agg in self._aggregates_spec)
        with create_query_span(
            operation="aggregate",
            table=table_name,
            filters_count=len(where_conditions) if where_conditions else 0,
            limit=self._limit_val if self._limit_val > 0 else None,
            aggregates=aggregates_info,
            group_by_count=len(self._group_by_cols) if self._group_by_cols else 0,
        ) as span:
            try:
                # Execute aggregate query
                result = await _engine.query_aggregate(
                    table=table_name,
                    aggregates=self._aggregates_spec,
                    group_by=self._group_by_cols if self._group_by_cols else None,
                    having=self._having_conditions if self._having_conditions else None,
                    where_conditions=where_conditions if where_conditions else None,
                    order_by=self._order_by_spec if self._order_by_spec else None,
                    limit=self._limit_val if self._limit_val > 0 else None,
                    distinct=self._distinct if self._distinct else None,
                    distinct_on=self._distinct_on_cols if self._distinct_on_cols else None,
                    ctes=self._ctes if self._ctes else None,
                    subqueries=self._subqueries if self._subqueries else None,
                    windows=self._windows if self._windows else None,
                    set_operations=self._set_operations if self._set_operations else None,
                )

                # Record result count
                set_span_result(span, count=len(result))
                return result
            except Exception as e:
                add_exception(span, e)
                raise

    async def to_list(self) -> List[T]:
        """
        Execute the query and return all matching rows.

        Returns:
            List of table instances

        Example:
            >>> users = await User.find(User.age > 25).to_list()
            >>> for user in users:
            ...     print(user.name)
            >>>
            >>> # With eager loading
            >>> from data_bridge.postgres import selectinload
            >>> posts = await Post.find().options(selectinload("author")).to_list()
            >>> for post in posts:
            ...     author = await post.author  # Already loaded, no query
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        # Check for CTEs - currently only supported with aggregate queries
        if self._ctes:
            raise NotImplementedError(
                "CTEs are currently only supported with aggregate() queries. "
                "Use .aggregate() instead of .to_list() when using CTEs."
            )

        table_name = self._model.__table_name__()

        # Build SQL query
        where_clause, params = self._build_where_clause()

        # Fast-path: no tracing overhead if disabled
        if not is_tracing_enabled():
            rows = await _engine.find_many(
                table_name,
                where_clause,
                params,
                self._order_by_spec,
                self._offset_val,
                self._limit_val,
                self._select_cols,
                self._distinct if self._distinct else None,
                self._distinct_on_cols if self._distinct_on_cols else None,
            )

            # Convert to model instances
            instances = [self._model(**row) for row in rows]

            # Apply eager loading options
            for option in self._options:
                await option.apply(instances)

            return instances

        # Create span with query attributes
        filters_count = len(self._filters) if self._filters else 0
        order_by_str = None
        if self._order_by_spec:
            order_by_str = ", ".join(f"{col} {direction}" for col, direction in self._order_by_spec)

        with create_query_span(
            operation="find",
            table=table_name,
            filters_count=filters_count,
            limit=self._limit_val if self._limit_val > 0 else None,
            offset=self._offset_val if self._offset_val > 0 else None,
            order_by=order_by_str,
        ) as span:
            try:
                # Execute query
                rows = await _engine.find_many(
                    table_name,
                    where_clause,
                    params,
                    self._order_by_spec,
                    self._offset_val,
                    self._limit_val,
                    self._select_cols,
                    self._distinct if self._distinct else None,
                    self._distinct_on_cols if self._distinct_on_cols else None,
                )

                # Convert to model instances
                instances = [self._model(**row) for row in rows]

                # Apply eager loading options
                for option in self._options:
                    await option.apply(instances)

                # Record result count
                set_span_result(span, count=len(instances))
                return instances
            except Exception as e:
                add_exception(span, e)
                raise

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
        # Fast-path: no tracing overhead if disabled
        if not is_tracing_enabled():
            result = await self._clone(_limit=1).to_list()
            return result[0] if result else None

        # Create span for find_one operation
        table_name = self._model.__table_name__()
        filters_count = len(self._filters) if self._filters else 0

        with create_query_span(
            operation="find_one",
            table=table_name,
            filters_count=filters_count,
            limit=1,
        ) as span:
            try:
                # Use limit(1) and return first result
                result = await self._clone(_limit=1).to_list()
                found = result[0] if result else None

                # Record whether a result was found
                set_span_result(span, count=1 if found else 0)
                return found
            except Exception as e:
                add_exception(span, e)
                raise

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

        # Fast-path: no tracing overhead if disabled
        if not is_tracing_enabled():
            return await _engine.count(table_name, where_clause, params)

        # Create span for count operation
        filters_count = len(self._filters) if self._filters else 0

        with create_query_span(
            operation="count",
            table=table_name,
            filters_count=filters_count,
        ) as span:
            try:
                # Execute count query
                result = await _engine.count(table_name, where_clause, params)

                # Record count result
                set_span_result(span, count=result)
                return result
            except Exception as e:
                add_exception(span, e)
                raise

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
        # Fast-path: no tracing overhead if disabled
        if not is_tracing_enabled():
            count = await self.count()
            return count > 0

        # Create span for exists operation
        table_name = self._model.__table_name__()
        filters_count = len(self._filters) if self._filters else 0

        with create_query_span(
            operation="exists",
            table=table_name,
            filters_count=filters_count,
        ) as span:
            try:
                count = await self.count()
                result = count > 0

                # Record whether any rows exist
                set_span_result(span, count=1 if result else 0)
                return result
            except Exception as e:
                add_exception(span, e)
                raise

    def _build_where_clause(self) -> tuple[str, list[Any]]:
        """
        Build WHERE clause from filters and JSON conditions.

        Returns:
            Tuple of (where_clause, parameters)
        """
        if not self._filters and not self._json_conditions:
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

        # Add JSON conditions
        for op_type, column, value in self._json_conditions:
            if op_type == "json_contains":
                conditions.append(f'"{column}" @> ${param_index}::jsonb')
                params.append(value)
                param_index += 1
            elif op_type == "json_contained_by":
                conditions.append(f'"{column}" <@ ${param_index}::jsonb')
                params.append(value)
                param_index += 1
            elif op_type == "json_key_exists":
                conditions.append(f'"{column}" ? ${param_index}')
                params.append(value)
                param_index += 1
            elif op_type == "json_any_key_exists":
                conditions.append(f'"{column}" ?| ${param_index}')
                params.append(value)
                param_index += 1
            elif op_type == "json_all_keys_exist":
                conditions.append(f'"{column}" ?& ${param_index}')
                params.append(value)
                param_index += 1

        where_clause = " AND ".join(conditions) if conditions else ""
        return (where_clause, params)
