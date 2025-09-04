from __future__ import annotations

from typing import TYPE_CHECKING, Any, TypeVar

from ...base.fields import CompoundExpression, QueryExpression
from ...base.manager import BaseManager
from .query import AsyncMongoQuery

if TYPE_CHECKING:
    from .document import AsyncDocument

T = TypeVar("T", bound="AsyncDocument")


class AsyncMongoCollection(BaseManager[T]):
    """MongoDB collection manager for asynchronous operations."""

    def find(
        self,
        *expressions: QueryExpression | CompoundExpression,
    ) -> AsyncMongoQuery[T]:
        """Create a MongoDB query with the given expressions.

        Examples:
            User.objects().find(User.age >= 18)
            User.objects().find(User.name == "John", User.active == True)
            User.objects().find((User.age >= 18) & (User.age <= 65))
        """
        return AsyncMongoQuery(self.model_class, expressions)

    async def find_one(
        self,
        *expressions: QueryExpression | CompoundExpression,
    ) -> T | None:
        """Find a single document matching the expressions."""
        query = AsyncMongoQuery(self.model_class, expressions)
        return await query.first()

    def all(self) -> AsyncMongoQuery[T]:
        """Return a query for all documents."""
        return AsyncMongoQuery(self.model_class, ())

    async def create(self, **kwargs: Any) -> T:
        """Create and save a new document instance."""
        instance = self.model_class(**kwargs)
        await instance.save()
        return instance

    async def get(self, **kwargs: Any) -> T:
        """Get a single document instance by field values."""
        expressions = self._create_field_expressions(**kwargs)
        query = AsyncMongoQuery(self.model_class, tuple(expressions))
        result = await query.first()
        if result is None:
            raise ValueError(f"No {self.model_class.__name__} found with {kwargs}")
        return result

    async def get_or_create(self, defaults: dict[str, Any] | None = None, **kwargs: Any) -> tuple[T, bool]:
        """Get or create a document instance."""
        try:
            return await self.get(**kwargs), False
        except ValueError:
            create_kwargs = {**kwargs, **(defaults or {})}
            return await self.create(**create_kwargs), True

    async def count(self) -> int:
        """Count all documents in the collection."""
        return await self.all().count()

    async def exists(self) -> bool:
        """Check if any documents exist."""
        return await self.count() > 0
