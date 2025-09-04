from __future__ import annotations

from collections.abc import Iterator, Sequence
from typing import TYPE_CHECKING, Any, TypeVar

from ...base.fields import CompoundExpression, QueryExpression
from ...base.query import BaseQuery

if TYPE_CHECKING:
    from .backend import MongoSyncBackend
    from .document import Document

T = TypeVar("T", bound="Document")


class MongoQuery(BaseQuery[T]):
    """MongoDB query builder for synchronous operations."""

    def __init__(
        self,
        model_class: type[T],
        expressions: Sequence[QueryExpression | CompoundExpression],
    ) -> None:
        super().__init__(model_class, expressions)
        self._backend: MongoSyncBackend | None = model_class._backend  # type: ignore[attr-defined]

    def filter(
        self,
        *expressions: QueryExpression | CompoundExpression,
    ) -> MongoQuery[T]:
        """Add additional filter expressions."""
        new_query = self._clone()
        new_query.expressions.extend(expressions)
        return new_query

    def limit(self, n: int) -> MongoQuery[T]:
        """Limit the number of results."""
        new_query = self._clone()
        new_query._limit_value = n
        return new_query

    def skip(self, n: int) -> MongoQuery[T]:
        """Skip the first n results."""
        new_query = self._clone()
        new_query._skip_value = n
        return new_query

    def sort(self, *fields: str | tuple[str, int]) -> MongoQuery[T]:
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

    def select(self, *fields: str) -> MongoQuery[T]:
        """Select specific fields to return (projection)."""
        new_query = self._clone()
        new_query._projection = list(fields)
        return new_query

    def execute(self) -> list[T]:
        """Execute the query and return results."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")
        return self._backend.execute_query(self)

    def first(self) -> T | None:
        """Get the first result or None."""
        results = self.limit(1).execute()
        return results[0] if results else None

    def count(self) -> int:
        """Count the number of matching documents."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")
        return self._backend.count_query(self)

    def exists(self) -> bool:
        """Check if any matching documents exist."""
        return self.limit(1).count() > 0

    def delete(self) -> int:
        """Delete all matching documents."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")
        return self._backend.delete_query(self)

    def update(self, **updates: Any) -> int:
        """Update all matching documents."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")
        return self._backend.update_query(self, updates)

    def __iter__(self) -> Iterator[T]:
        """Iterate over query results."""
        return iter(self.execute())

    def __getitem__(self, key: int | slice) -> T | list[T]:
        """Support indexing and slicing."""
        if isinstance(key, int):
            if key < 0:
                raise ValueError("Negative indexing not supported")
            results = self.skip(key).limit(1).execute()
            if not results:
                raise IndexError("Query index out of range")
            return results[0]
        elif isinstance(key, slice):
            start = key.start or 0
            stop = key.stop
            if key.step is not None:
                raise ValueError("Step not supported in query slicing")
            if start < 0 or (stop is not None and stop < 0):
                raise ValueError("Negative indexing not supported")

            query = self.skip(start)
            if stop is not None:
                query = query.limit(stop - start)
            return query.execute()
        else:
            raise TypeError(f"Invalid index type: {type(key)}")

    def _clone(self) -> MongoQuery[T]:
        """Create a copy of this query."""
        new_query = MongoQuery(self.model_class, self.expressions)
        new_query._limit_value = self._limit_value
        new_query._skip_value = self._skip_value
        new_query._sort_fields = self._sort_fields.copy()
        new_query._projection = self._projection.copy() if self._projection else None
        new_query._backend = self._backend
        return new_query
