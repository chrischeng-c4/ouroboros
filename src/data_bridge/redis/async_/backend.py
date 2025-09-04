from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any, TypeVar, Union

import redis.asyncio as aioredis

from ...base.backends.async_ import AsyncBackend
from ..key_patterns import RedisKeyPattern

if TYPE_CHECKING:
    from .hash_model import AsyncHashModel
    from .json_model import AsyncJSONModel

T = TypeVar("T", bound=Union["AsyncHashModel", "AsyncJSONModel"])


class RedisAsyncBackend(AsyncBackend):
    """Asynchronous Redis backend implementation."""

    def __init__(
        self,
        connection_string: str = "redis://localhost:6379",
        **redis_kwargs: Any
    ) -> None:
        """Initialize the Redis backend.

        Args:
            connection_string: Redis connection string
            **redis_kwargs: Additional arguments passed to aioredis
        """
        self.connection_string = connection_string
        self.redis_kwargs = redis_kwargs
        self.client: aioredis.Redis | None = None

    async def connect(self) -> None:
        """Establish connection to Redis."""
        if self.client is None:
            self.client = aioredis.from_url(self.connection_string, **self.redis_kwargs)

    async def disconnect(self) -> None:
        """Close connection to Redis."""
        if self.client:
            await self.client.aclose()
            self.client = None

    async def _get_client(self) -> aioredis.Redis:
        """Get Redis client, connecting if necessary."""
        if self.client is None:
            await self.connect()
        return self.client  # type: ignore[return-value]

    async def save(self, instance: T, ttl: int | None = None) -> None:
        """Save a model instance to Redis."""
        client = await self._get_client()
        key = instance.get_key()
        data = instance.to_dict()

        # Simple implementation - store as JSON string
        json_str = json.dumps(data)
        if ttl:
            await client.setex(key, ttl, json_str)
        else:
            await client.set(key, json_str)

    async def delete(self, instance: T) -> None:
        """Delete a model instance from Redis."""
        client = await self._get_client()
        key = instance.get_key()
        await client.delete(key)

    async def get_by_key(self, model_class: type[T], primary_key: Any) -> T | None:
        """Get a model instance by primary key."""
        client = await self._get_client()
        key = RedisKeyPattern.build_key(model_class._key_prefix, primary_key)

        json_str = await client.get(key)
        if json_str is None:
            return None

        data = json.loads(json_str)
        return model_class.from_dict(data)

    # Simplified stubs for base class requirements
    async def execute_query(self, query) -> list[T]:
        """Execute a query (simplified implementation)."""
        return []

    async def count_query(self, query) -> int:
        """Count matching documents (simplified implementation)."""
        return 0

    async def delete_query(self, query) -> int:
        """Delete matching documents (simplified implementation)."""
        return 0

    async def update_query(self, query, updates: dict[str, Any]) -> int:
        """Update matching documents (simplified implementation)."""
        return 0
