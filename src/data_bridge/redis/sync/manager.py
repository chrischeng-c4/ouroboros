from __future__ import annotations

from typing import TYPE_CHECKING, Any, TypeVar, Union

from ...base.fields import CompoundExpression, QueryExpression
from ...base.manager import BaseManager

if TYPE_CHECKING:
    from .hash_model import HashModel
    from .json_model import JSONModel

T = TypeVar("T", bound=Union["HashModel", "JSONModel"])


class RedisManager(BaseManager[T]):
    """Redis manager for both HashModel and JSONModel."""

    def find(
        self,
        *expressions: QueryExpression | CompoundExpression,
    ) -> list[T]:
        """Find models matching the expressions.

        Note: Redis has limited query capabilities compared to document databases.
        This method will scan through keys and filter in memory for now.
        For JSONModel, consider using the find() class method with JSONPath filters.
        """
        if not self.model_class._backend:  # type: ignore[attr-defined]
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")

        # This is a basic implementation that scans all keys
        # In a production system, you might want to use Redis Search module
        # or implement more efficient indexing
        return self.model_class._backend.find_with_expressions(  # type: ignore[attr-defined]
            self.model_class, list(expressions)
        )

    def all(self) -> list[T]:
        """Return all models of this type.

        Warning: This will scan all keys with the model's prefix.
        Use with caution in production with large datasets.
        """
        if not self.model_class._backend:  # type: ignore[attr-defined]
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")

        return self.model_class._backend.get_all(self.model_class)  # type: ignore[attr-defined]

    def create(self, ttl: int | None = None, **kwargs: Any) -> T:
        """Create and save a new model instance.

        Args:
            ttl: Time to live in seconds
            **kwargs: Field values for the model
        """
        instance = self.model_class(**kwargs)
        instance.save(ttl=ttl)  # type: ignore[call-arg]
        return instance

    def get(self, **kwargs: Any) -> T:
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
                result = self.model_class.get(kwargs[pk_field_name])  # type: ignore[attr-defined]
                if result is None:
                    raise ValueError(f"No {self.model_class.__name__} found with {kwargs}")
                return result

        # Fall back to expression-based search
        expressions = self._create_field_expressions(**kwargs)
        results = self.find(*expressions)
        if not results:
            raise ValueError(f"No {self.model_class.__name__} found with {kwargs}")
        if len(results) > 1:
            raise ValueError(f"Multiple {self.model_class.__name__} found with {kwargs}")
        return results[0]

    def get_or_create(
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
            return self.get(**kwargs), False
        except ValueError:
            create_kwargs = {**kwargs, **(defaults or {})}
            return self.create(ttl=ttl, **create_kwargs), True

    def count(self) -> int:
        """Count all models of this type.

        Warning: This will scan all keys with the model's prefix.
        """
        return len(self.all())

    def exists(self) -> bool:
        """Check if any models of this type exist."""
        if not self.model_class._backend:  # type: ignore[attr-defined]
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")

        return self.model_class._backend.any_exists(self.model_class)  # type: ignore[attr-defined]

    def delete_all(self) -> int:
        """Delete all models of this type.

        Warning: This will delete all keys with the model's prefix.
        Use with extreme caution.
        """
        if not self.model_class._backend:  # type: ignore[attr-defined]
            raise RuntimeError(f"No backend configured for {self.model_class.__name__}")

        return self.model_class._backend.delete_all(self.model_class)  # type: ignore[attr-defined]
