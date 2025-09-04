from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar, TypeVar

from ...base.model import BaseModel
from ..key_patterns import RedisKeyPattern

if TYPE_CHECKING:
    from .backend import RedisSyncBackend
    from .manager import RedisManager

T = TypeVar("T", bound="HashModel")


class HashModel(BaseModel):
    """Redis Hash-based model for synchronous operations."""

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
        """Save the model instance to Redis as a hash.

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

    def get_field(self, field_name: str) -> Any:
        """Get a single field value from Redis hash (efficient)."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        return self._backend.get_field(self, field_name)

    def set_field(self, field_name: str, value: Any, ttl: int | None = None) -> None:
        """Set a single field value in Redis hash (efficient).

        Args:
            field_name: Name of the field to set
            value: Value to set
            ttl: Time to live in seconds. If None, uses existing TTL
        """
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        self._backend.set_field(self, field_name, value, ttl)
        # Update the local instance
        setattr(self, field_name, value)

    @classmethod
    def get(cls: type[T], primary_key: Any) -> T | None:
        """Get a model instance by primary key."""
        if not cls._backend:
            raise RuntimeError(f"No backend configured for {cls.__name__}")
        return cls._backend.get_by_key(cls, primary_key)

    def exists(self) -> bool:
        """Check if this model instance exists in Redis."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        return self._backend.exists(self)
