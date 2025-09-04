from __future__ import annotations

from typing import TYPE_CHECKING, Any, TypeVar

from ...base.fields import CompoundExpression, QueryExpression
from ...base.manager import BaseManager
from .query import MongoQuery

if TYPE_CHECKING:
    from .document import Document

T = TypeVar("T", bound="Document")


class MongoCollection(BaseManager[T]):
    """MongoDB collection manager for synchronous operations."""

    def find(
        self,
        *expressions: QueryExpression | CompoundExpression,
    ) -> MongoQuery[T]:
        """Create a MongoDB query with the given expressions.

        Examples:
            User.objects().find(User.age >= 18)
            User.objects().find(User.name == "John", User.active == True)
            User.objects().find((User.age >= 18) & (User.age <= 65))
        """
        return MongoQuery(self.model_class, expressions)

    def find_one(
        self,
        *expressions: QueryExpression | CompoundExpression,
    ) -> T | None:
        """Find a single document matching the expressions."""
        query = MongoQuery(self.model_class, expressions)
        return query.first()

    def all(self) -> MongoQuery[T]:
        """Return a query for all documents."""
        return MongoQuery(self.model_class, ())

    def create(self, **kwargs: Any) -> T:
        """Create and save a new document instance."""
        instance = self.model_class(**kwargs)
        instance.save()
        return instance

    def get(self, **kwargs: Any) -> T:
        """Get a single document instance by field values."""
        expressions = self._create_field_expressions(**kwargs)
        query = MongoQuery(self.model_class, tuple(expressions))
        result = query.first()
        if result is None:
            raise ValueError(f"No {self.model_class.__name__} found with {kwargs}")
        return result

    def get_or_create(self, defaults: dict[str, Any] | None = None, **kwargs: Any) -> tuple[T, bool]:
        """Get or create a document instance."""
        try:
            return self.get(**kwargs), False
        except ValueError:
            create_kwargs = {**kwargs, **(defaults or {})}
            return self.create(**create_kwargs), True

    def count(self) -> int:
        """Count all documents in the collection."""
        return self.all().count()

    def exists(self) -> bool:
        """Check if any documents exist."""
        return self.count() > 0
