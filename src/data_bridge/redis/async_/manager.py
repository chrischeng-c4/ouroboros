from __future__ import annotations

from typing import TYPE_CHECKING, Any, TypeVar, Union

from ...base.fields import CompoundExpression, QueryExpression
from ...base.manager import BaseManager

if TYPE_CHECKING:
    from .hash_model import AsyncHashModel
    from .json_model import AsyncJSONModel

T = TypeVar("T", bound=Union["AsyncHashModel", "AsyncJSONModel"])


class AsyncRedisManager(BaseManager[T]):
    """Async Redis manager for both AsyncHashModel and AsyncJSONModel."""

    async def create(self, ttl: int | None = None, **kwargs: Any) -> T:
        """Create and save a new model instance.

        Args:
            ttl: Time to live in seconds
            **kwargs: Field values for the model
        """
        instance = self.model_class(**kwargs)
        await instance.save(ttl=ttl)  # type: ignore[call-arg]
        return instance

    async def get(self, **kwargs: Any) -> T:
        """Get a single model instance by field values.

        For Redis models, this is most efficient when querying by primary key.
        """
        if len(kwargs) == 1:
            # Check if it's a primary key lookup
            pk_field_name = next(
                (name for name, field in self.model_class._fields.items()  # type: ignore[attr-defined]
                 if field.primary_key),
                None
            )
            if pk_field_name and pk_field_name in kwargs:
                result = await self.model_class.get(kwargs[pk_field_name])  # type: ignore[attr-defined]
                if result is None:
                    raise ValueError(f"No {self.model_class.__name__} found with {kwargs}")
                return result

        raise NotImplementedError("Complex queries not implemented for Redis async models")

    async def get_or_create(
        self,
        defaults: dict[str, Any] | None = None,
        ttl: int | None = None,
        **kwargs: Any
    ) -> tuple[T, bool]:
        """Get or create a model instance.

        Args:
            defaults: Default values to use when creating
            ttl: Time to live in seconds for new instances
            **kwargs: Field values to search for
        """
        try:
            return await self.get(**kwargs), False
        except ValueError:
            create_kwargs = {**kwargs, **(defaults or {})}
            return await self.create(ttl=ttl, **create_kwargs), True

    # Simplified stubs for base class requirements
    def find(self, *expressions: QueryExpression | CompoundExpression):
        raise NotImplementedError("Query-based find not implemented for async Redis models")

    def all(self):
        raise NotImplementedError("All query not implemented for async Redis models")
