from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar, TypeVar

from ...base.model import BaseModel
from ..key_patterns import RedisKeyPattern

if TYPE_CHECKING:
    from .backend import RedisAsyncBackend
    from .manager import AsyncRedisManager

T = TypeVar("T", bound="AsyncJSONModel")


class AsyncJSONModel(BaseModel):
    """Redis JSON-based model for asynchronous operations."""

    _key_prefix: ClassVar[str]
    _default_ttl: ClassVar[int | None] = None
    _backend: ClassVar[RedisAsyncBackend | None] = None

    @classmethod
    def objects(cls: type[T]) -> AsyncRedisManager[T]:
        """Return a manager instance for this model."""
        from .manager import AsyncRedisManager
        return AsyncRedisManager(cls)

    @classmethod
    def set_backend(cls, backend: RedisAsyncBackend) -> None:
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

    async def save(self, ttl: int | None = None) -> None:
        """Save the model instance to Redis as JSON.

        Args:
            ttl: Time to live in seconds. If None, uses _default_ttl
        """
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")

        effective_ttl = ttl or self._default_ttl
        await self._backend.save(self, effective_ttl)

    async def delete(self) -> None:
        """Delete the model instance from Redis."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        await self._backend.delete(self)

    @classmethod
    async def get(cls: type[T], primary_key: Any) -> T | None:
        """Get a model instance by primary key."""
        if not cls._backend:
            raise RuntimeError(f"No backend configured for {cls.__name__}")
        return await cls._backend.get_by_key(cls, primary_key)
