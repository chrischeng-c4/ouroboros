"""
KV Store client for data-bridge.

High-performance key-value store client that connects to kv-server via TCP.

Example:
    >>> from data_bridge.kv import KvClient
    >>>
    >>> async with KvClient.connect("127.0.0.1:6380") as client:
    ...     await client.set("key", "value")
    ...     result = await client.get("key")
    ...     print(result)
    'value'
"""

from __future__ import annotations

from typing import Any, Optional, Union
from decimal import Decimal

# Import from Rust bindings
try:
    from data_bridge.data_bridge import kv as _kv
    _KvClient = _kv.KvClient
except ImportError:
    # KV feature not enabled
    _KvClient = None

__all__ = ["KvClient", "KvValue"]

# Type alias for supported value types
KvValue = Union[None, int, float, Decimal, str, bytes, list, dict]


class KvClient:
    """
    Async KV store client.

    Connects to a kv-server instance via TCP and provides high-performance
    key-value operations.

    Attributes:
        _client: The underlying Rust client instance.
    """

    __slots__ = ("_client",)

    def __init__(self, client: Any) -> None:
        """Initialize with an existing client instance."""
        self._client = client

    @classmethod
    async def connect(cls, addr: str = "127.0.0.1:6380") -> "KvClient":
        """
        Connect to a KV server.

        Args:
            addr: Server address in "host:port" format.
                  Defaults to "127.0.0.1:6380".

        Returns:
            Connected KvClient instance.

        Raises:
            ConnectionError: If connection fails.

        Example:
            >>> client = await KvClient.connect("localhost:6380")
        """
        if _KvClient is None:
            raise ImportError(
                "KV module not available. "
                "Rebuild with: maturin develop --features kv"
            )
        client = await _KvClient.connect(addr)
        return cls(client)

    async def __aenter__(self) -> "KvClient":
        """Async context manager entry."""
        return self

    async def __aexit__(self, *args: Any) -> None:
        """Async context manager exit."""
        # Connection cleanup handled by Rust
        pass

    async def ping(self) -> str:
        """
        Ping the server.

        Returns:
            "PONG" if server is responsive.

        Raises:
            ConnectionError: If server is unreachable.
        """
        return await self._client.ping()

    async def get(self, key: str) -> Optional[KvValue]:
        """
        Get a value by key.

        Args:
            key: The key to look up (max 256 characters).

        Returns:
            The value if found, None otherwise.

        Example:
            >>> value = await client.get("mykey")
            >>> if value is not None:
            ...     print(f"Found: {value}")
        """
        return await self._client.get(key)

    async def set(
        self,
        key: str,
        value: KvValue,
        ttl: Optional[float] = None,
    ) -> None:
        """
        Set a value.

        Args:
            key: The key to set (max 256 characters).
            value: The value to store. Supported types:
                   int, float, Decimal, str, bytes, list, dict, None
            ttl: Optional time-to-live in seconds.

        Example:
            >>> await client.set("name", "Alice")
            >>> await client.set("counter", 0)
            >>> await client.set("temp", 123, ttl=60)  # Expires in 60s
        """
        await self._client.set(key, value, ttl)

    async def delete(self, key: str) -> bool:
        """
        Delete a key.

        Args:
            key: The key to delete.

        Returns:
            True if the key existed, False otherwise.

        Example:
            >>> deleted = await client.delete("mykey")
            >>> print(f"Key was {'deleted' if deleted else 'not found'}")
        """
        return await self._client.delete(key)

    async def exists(self, key: str) -> bool:
        """
        Check if a key exists.

        Args:
            key: The key to check.

        Returns:
            True if the key exists (and not expired).

        Example:
            >>> if await client.exists("mykey"):
            ...     print("Key exists!")
        """
        return await self._client.exists(key)

    async def incr(self, key: str, delta: int = 1) -> int:
        """
        Atomically increment an integer value.

        If the key doesn't exist, it's created with the delta as initial value.

        Args:
            key: The key to increment.
            delta: Amount to add (default: 1).

        Returns:
            The new value after incrementing.

        Raises:
            TypeError: If the existing value is not an integer.

        Example:
            >>> await client.set("counter", 10)
            >>> new_value = await client.incr("counter", 5)
            >>> print(new_value)  # 15
        """
        return await self._client.incr(key, delta)

    async def decr(self, key: str, delta: int = 1) -> int:
        """
        Atomically decrement an integer value.

        Args:
            key: The key to decrement.
            delta: Amount to subtract (default: 1).

        Returns:
            The new value after decrementing.

        Raises:
            TypeError: If the existing value is not an integer.

        Example:
            >>> await client.set("counter", 10)
            >>> new_value = await client.decr("counter", 3)
            >>> print(new_value)  # 7
        """
        return await self._client.decr(key, delta)

    async def info(self) -> str:
        """
        Get server information.

        Returns:
            JSON string with server statistics.

        Example:
            >>> info = await client.info()
            >>> print(info)
            {"shards": 256, "entries": 1000}
        """
        return await self._client.info()

    def __repr__(self) -> str:
        return "KvClient(connected)"
