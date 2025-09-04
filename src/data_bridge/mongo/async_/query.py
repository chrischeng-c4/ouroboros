from __future__ import annotations

from collections.abc import AsyncIterator, Sequence
from typing import TYPE_CHECKING, Any, TypeVar

from ...base.fields import CompoundExpression, QueryExpression
from ...base.query import BaseQuery

if TYPE_CHECKING:
    from .backend import MongoAsyncBackend
    from .document import AsyncDocument

T = TypeVar("T", bound="AsyncDocument")


class AsyncMongoQuery(BaseQuery[T]):
    """MongoDB query builder for asynchronous operations."""

    def __init__(
        self,
        model_class: type[T],
        expressions: Sequence[QueryExpression | CompoundExpression],
    ) -> None:
        super().__init__(model_class, expressions)
        self._backend: MongoAsyncBackend | None = model_class._backend  # type: ignore[attr-defined]

    def filter(
        self,
        *expressions: QueryExpression | CompoundExpression,
    ) -> AsyncMongoQuery[T]:
        """Add additional filter expressions."""
        new_query = self._clone()
        new_query.expressions.extend(expressions)
        return new_query

    def limit(self, n: int) -> AsyncMongoQuery[T]:
        """Limit the number of results."""
        new_query = self._clone()
        new_query._limit_value = n
        return new_query

    def skip(self, n: int) -> AsyncMongoQuery[T]:
        """Skip the first n results."""
        new_query = self._clone()
        new_query._skip_value = n
        return new_query

    def sort(self, *fields: str | tuple[str, int]) -> AsyncMongoQuery[T]:
        """Sort results by field(s).

        Args:
            fields: Field names or tuples of (field_name, direction)
                   where direction is 1 for ascending, -1 for descending

        Examples:
            query.sort("name")  # Sort by name ascending
            query.sort("-age")  # Sort by age descending
            query.sort(("name", 1), ("age", -1))  # Sort by name asc, then age desc
        """
        new_query = self._clone()
        new_query._sort_fields.extend(self._parse_sort_fields(*fields))
        return new_query

    def select(self, *fields: str) -> AsyncMongoQuery[T]:
        """Select specific fields to return (projection)."""
        new_query = self._clone()
        new_query._projection = list(fields)
        return new_query

    async def execute(self) -> list[T]:
        """Execute the query and return results."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")
        return await self._backend.execute_query(self)

    async def first(self) -> T | None:
        """Get the first result or None."""
        results = await self.limit(1).execute()
        return results[0] if results else None

    async def count(self) -> int:
        """Count the number of matching documents."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")
        return await self._backend.count_query(self)

    async def exists(self) -> bool:
        """Check if any matching documents exist."""
        return await self.limit(1).count() > 0

    async def delete(self) -> int:
        """Delete all matching documents."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")
        return await self._backend.delete_query(self)

    async def update(self, **updates: Any) -> int:
        """Update all matching documents."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")
        return await self._backend.update_query(self, updates)

    async def __aiter__(self) -> AsyncIterator[T]:
        """Async iterate over query results."""
        results = await self.execute()
        for result in results:
            yield result

    def _clone(self) -> AsyncMongoQuery[T]:
        """Create a copy of this query."""
        new_query = AsyncMongoQuery(self.model_class, self.expressions)
        new_query._limit_value = self._limit_value
        new_query._skip_value = self._skip_value
        new_query._sort_fields = self._sort_fields.copy()
        new_query._projection = self._projection.copy() if self._projection else None
        new_query._backend = self._backend
        return new_query
