from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any, TypeVar, Union

import redis

from ...base.backends.sync import SyncBackend
from ..key_patterns import RedisKeyPattern

if TYPE_CHECKING:
    from .hash_model import HashModel
    from .json_model import JSONModel

T = TypeVar("T", bound=Union["HashModel", "JSONModel"])


class RedisSyncBackend(SyncBackend):
    """Synchronous Redis backend implementation."""

    def __init__(
        self,
        connection_string: str = "redis://localhost:6379",
        **redis_kwargs: Any
    ) -> None:
        """Initialize the Redis backend.

        Args:
            connection_string: Redis connection string
            **redis_kwargs: Additional arguments passed to Redis client
        """
        self.connection_string = connection_string
        self.redis_kwargs = redis_kwargs
        self.client: redis.Redis | None = None

    def connect(self) -> None:
        """Establish connection to Redis."""
        if self.client is None:
            self.client = redis.from_url(self.connection_string, **self.redis_kwargs)

    def disconnect(self) -> None:
        """Close connection to Redis."""
        if self.client:
            self.client.close()
            self.client = None

    def _get_client(self) -> redis.Redis:
        """Get Redis client, connecting if necessary."""
        if self.client is None:
            self.connect()
        return self.client  # type: ignore[return-value]

    def save(self, instance: T, ttl: int | None = None) -> None:
        """Save a model instance to Redis."""
        client = self._get_client()
        key = instance.get_key()

        # Determine if this is a HashModel or JSONModel
        from .hash_model import HashModel
        from .json_model import JSONModel

        if isinstance(instance, HashModel):
            self._save_hash_model(client, instance, key, ttl)
        elif isinstance(instance, JSONModel):
            self._save_json_model(client, instance, key, ttl)
        else:
            raise ValueError(f"Unknown model type: {type(instance)}")

    def _save_hash_model(self, client: redis.Redis, instance: HashModel, key: str, ttl: int | None) -> None:
        """Save a HashModel instance using Redis HASH commands."""
        data = instance.to_dict()

        # Convert all values to strings for Redis hash storage
        hash_data = {}
        for field_name, value in data.items():
            if value is not None:
                if isinstance(value, dict | list):
                    # Serialize complex types as JSON for hash storage
                    hash_data[field_name] = json.dumps(value)
                else:
                    hash_data[field_name] = str(value)

        # Use pipeline for atomicity
        with client.pipeline() as pipe:
            pipe.hset(key, mapping=hash_data)
            if ttl:
                pipe.expire(key, ttl)
            pipe.execute()

    def _save_json_model(self, client: redis.Redis, instance: JSONModel, key: str, ttl: int | None) -> None:
        """Save a JSONModel instance using Redis JSON commands."""
        data = instance.to_dict()

        try:
            # Try to use RedisJSON commands
            client.json().set(key, "$", data)
            if ttl:
                client.expire(key, ttl)
        except AttributeError:
            # Fall back to regular JSON string storage if RedisJSON not available
            json_str = json.dumps(data)
            if ttl:
                client.setex(key, ttl, json_str)
            else:
                client.set(key, json_str)

    def delete(self, instance: T) -> None:
        """Delete a model instance from Redis."""
        client = self._get_client()
        key = instance.get_key()
        client.delete(key)

    def get_by_key(self, model_class: type[T], primary_key: Any) -> T | None:
        """Get a model instance by primary key."""
        client = self._get_client()
        key = RedisKeyPattern.build_key(model_class._key_prefix, primary_key)

        from .hash_model import HashModel
        from .json_model import JSONModel

        if issubclass(model_class, HashModel):
            return self._get_hash_model(client, model_class, key)
        elif issubclass(model_class, JSONModel):
            return self._get_json_model(client, model_class, key)
        else:
            raise ValueError(f"Unknown model type: {model_class}")

    def _get_hash_model(self, client: redis.Redis, model_class: type[T], key: str) -> T | None:
        """Get a HashModel instance from Redis."""
        data = client.hgetall(key)
        if not data:
            return None

        # Convert bytes keys and values back to strings/proper types
        converted_data = {}
        for field_name, field in model_class._fields.items():
            if field_name.encode() in data:
                raw_value = data[field_name.encode()].decode()

                # Convert back to proper type
                if field.type_ == str:
                    converted_data[field_name] = raw_value
                elif field.type_ == int:
                    converted_data[field_name] = int(raw_value)
                elif field.type_ == float:
                    converted_data[field_name] = float(raw_value)
                elif field.type_ == bool:
                    converted_data[field_name] = raw_value.lower() == 'true'
                elif field.type_ in (dict, list):
                    converted_data[field_name] = json.loads(raw_value)
                else:
                    converted_data[field_name] = raw_value

        return model_class.from_dict(converted_data)

    def _get_json_model(self, client: redis.Redis, model_class: type[T], key: str) -> T | None:
        """Get a JSONModel instance from Redis."""
        try:
            # Try RedisJSON first
            data = client.json().get(key)
            if data is None:
                return None
        except AttributeError:
            # Fall back to regular string storage
            json_str = client.get(key)
            if json_str is None:
                return None
            data = json.loads(json_str)

        return model_class.from_dict(data)

    def exists(self, instance: T) -> bool:
        """Check if a model instance exists in Redis."""
        client = self._get_client()
        key = instance.get_key()
        return bool(client.exists(key))

    def get_field(self, instance: HashModel, field_name: str) -> Any:
        """Get a single field value from a HashModel (efficient for Redis HASH)."""
        client = self._get_client()
        key = instance.get_key()
        raw_value = client.hget(key, field_name)

        if raw_value is None:
            return None

        # Convert to proper type based on field definition
        field = instance._fields.get(field_name)
        if field and field.type_:
            if field.type_ == str:
                return raw_value.decode()
            elif field.type_ == int:
                return int(raw_value.decode())
            elif field.type_ == float:
                return float(raw_value.decode())
            elif field.type_ == bool:
                return raw_value.decode().lower() == 'true'
            elif field.type_ in (dict, list):
                return json.loads(raw_value.decode())

        return raw_value.decode()

    def set_field(self, instance: HashModel, field_name: str, value: Any, ttl: int | None = None) -> None:
        """Set a single field value in a HashModel (efficient for Redis HASH)."""
        client = self._get_client()
        key = instance.get_key()

        # Convert value to string for Redis storage
        str_value = json.dumps(value) if isinstance(value, dict | list) else str(value)

        with client.pipeline() as pipe:
            pipe.hset(key, field_name, str_value)
            if ttl:
                pipe.expire(key, ttl)
            pipe.execute()

    # Simplified implementations for other methods
    def execute_query(self, query) -> list[T]:
        """Execute a query (simplified implementation)."""
        # For now, return empty list as full query implementation is complex
        return []

    def count_query(self, query) -> int:
        """Count matching documents (simplified implementation)."""
        return 0

    def delete_query(self, query) -> int:
        """Delete matching documents (simplified implementation)."""
        return 0

    def update_query(self, query, updates: dict[str, Any]) -> int:
        """Update matching documents (simplified implementation)."""
        return 0
