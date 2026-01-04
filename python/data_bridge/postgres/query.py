"""
Query builder for chainable PostgreSQL queries.

This module provides a query builder that supports:
- Fluent/chainable API: .order_by().offset().limit().to_list()
- Async execution with Rust backend
- Type-safe query expressions
- Aggregation queries: .sum().avg().count_agg().group_by().having().aggregate()
- Common Table Expressions (CTEs): .with_cte().with_cte_raw().from_cte()

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

    def _clone(self, **kwargs: Any) -> "QueryBuilder[T]":
        """Create a copy of this builder with updated values."""
        return QueryBuilder(
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

        # Execute aggregate query
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
        )

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

        # Check for CTEs - currently only supported with aggregate queries
        if self._ctes:
            raise NotImplementedError(
                "CTEs are currently only supported with aggregate() queries. "
                "Use .aggregate() instead of .to_list() when using CTEs."
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
            self._distinct if self._distinct else None,
            self._distinct_on_cols if self._distinct_on_cols else None,
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
