from __future__ import annotations

from typing import Any


class RedisKeyPattern:
    """Utilities for Redis key pattern management."""

    @staticmethod
    def build_key(prefix: str, primary_key: Any) -> str:
        """Build a Redis key from prefix and primary key."""
        return f"{prefix}{primary_key}"

    @staticmethod
    def parse_key(key: str, prefix: str) -> str:
        """Extract primary key from Redis key."""
        if not key.startswith(prefix):
            raise ValueError(f"Key '{key}' does not match prefix '{prefix}'")
        return key[len(prefix):]

    @staticmethod
    def build_pattern(prefix: str) -> str:
        """Build a Redis key pattern for scanning."""
        return f"{prefix}*"

    @staticmethod
    def validate_prefix(prefix: str) -> None:
        """Validate that a prefix is suitable for Redis keys."""
        if not prefix:
            raise ValueError("Prefix cannot be empty")
        if not prefix.endswith(":"):
            raise ValueError("Prefix should end with ':'")
        if " " in prefix:
            raise ValueError("Prefix should not contain spaces")
