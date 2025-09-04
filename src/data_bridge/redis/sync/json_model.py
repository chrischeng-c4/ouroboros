from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar, TypeVar

from ...base.model import BaseModel
from ..key_patterns import RedisKeyPattern

if TYPE_CHECKING:
    from .backend import RedisSyncBackend
    from .manager import RedisManager

T = TypeVar("T", bound="JSONModel")


class JSONModel(BaseModel):
    """Redis JSON-based model for synchronous operations."""

    _key_prefix: ClassVar[str]
    _default_ttl: ClassVar[int | None] = None
    _backend: ClassVar[RedisSyncBackend | None] = None

    @classmethod
    def objects(cls: type[T]) -> RedisManager[T]:
        """Return a manager instance for this model."""
        from .manager import RedisManager
        return RedisManager(cls)

    @classmethod
    def set_backend(cls, backend: RedisSyncBackend) -> None:
        """Set the Redis backend for this model."""
        cls._backend = backend

    def get_key(self) -> str:
        """Get the Redis key for this model instance."""
        if not hasattr(self, '_pk_field') or not self._pk_field:
            raise ValueError("Cannot generate key without primary key field")

        pk_field_name = next(
            name for name, field in self._fields.items()
            if field.primary_key
        )
        pk_value = getattr(self, pk_field_name)
        if pk_value is None:
            raise ValueError("Cannot generate key with None primary key")

        return RedisKeyPattern.build_key(self._key_prefix, pk_value)

    def save(self, ttl: int | None = None) -> None:
        """Save the model instance to Redis as JSON.

        Args:
            ttl: Time to live in seconds. If None, uses _default_ttl
        """
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")

        effective_ttl = ttl or self._default_ttl
        self._backend.save(self, effective_ttl)

    def delete(self) -> None:
        """Delete the model instance from Redis."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        self._backend.delete(self)

    def update_path(self, json_path: str, value: Any) -> None:
        """Update a value at a specific JSONPath.

        Args:
            json_path: JSONPath expression (e.g., "$.customer.email")
            value: Value to set at the path
        """
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        self._backend.update_json_path(self, json_path, value)
        # Note: We don't update the local instance here since JSONPath
        # updates can be complex and we'd need to parse the path

    def get_path(self, json_path: str) -> Any:
        """Get a value at a specific JSONPath.

        Args:
            json_path: JSONPath expression (e.g., "$.customer.email")

        Returns:
            The value at the specified path
        """
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        return self._backend.get_json_path(self, json_path)

    @classmethod
    def get(cls: type[T], primary_key: Any) -> T | None:
        """Get a model instance by primary key."""
        if not cls._backend:
            raise RuntimeError(f"No backend configured for {cls.__name__}")
        return cls._backend.get_by_key(cls, primary_key)

    @classmethod
    def find(cls: type[T], json_filters: dict) -> list[T]:
        """Find models using JSONPath filters.

        Args:
            json_filters: Dictionary of JSONPath expressions to values
                         e.g., {"$.status": "pending", "$.total": {"$gt": 100}}

        Returns:
            List of matching model instances
        """
        if not cls._backend:
            raise RuntimeError(f"No backend configured for {cls.__name__}")
        return cls._backend.find_by_json_filter(cls, json_filters)

    def exists(self) -> bool:
        """Check if this model instance exists in Redis."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        return self._backend.exists(self)
