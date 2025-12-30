"""Transaction support for PostgreSQL.

This module provides transaction management for PostgreSQL operations.

Example:
    >>> from data_bridge.postgres import pg_transaction
    >>>
    >>> async with pg_transaction() as tx:
    ...     user = User(name="Alice", email="alice@example.com")
    ...     await user.save()
    ...     order = Order(user_id=user.id, amount=100.0)
    ...     await order.save()
    ...     # Transaction commits on exit
    ...     # Or rolls back on exception
"""

from __future__ import annotations

from contextlib import asynccontextmanager
from typing import Any, AsyncGenerator, Optional

# Import from Rust engine when available
try:
    from data_bridge.data_bridge import postgres as _engine
except ImportError:
    _engine = None


class Transaction:
    """
    PostgreSQL transaction context.

    Transactions provide ACID guarantees for multi-statement operations.

    Example:
        >>> async with pg_transaction() as tx:
        ...     await user.save()
        ...     await order.save()
        ...     # Commits on successful exit
        ...     # Rolls back on exception
    """

    def __init__(self, tx_id: Any):
        """
        Initialize transaction context.

        Args:
            tx_id: Transaction identifier from Rust backend
        """
        self._tx_id = tx_id
        self._committed = False
        self._rolled_back = False

    @property
    def is_active(self) -> bool:
        """Check if transaction is still active."""
        return not self._committed and not self._rolled_back

    async def commit(self) -> None:
        """
        Manually commit the transaction.

        This is usually not needed as the transaction commits automatically
        on successful exit from the context manager.

        Raises:
            RuntimeError: If transaction is not active or engine not available
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        if not self.is_active:
            raise RuntimeError("Transaction is not active")

        await _engine.commit_transaction(self._tx_id)
        self._committed = True

    async def rollback(self) -> None:
        """
        Manually rollback the transaction.

        This is usually not needed as the transaction rolls back automatically
        on exception.

        Raises:
            RuntimeError: If transaction is not active or engine not available
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        if not self.is_active:
            raise RuntimeError("Transaction is not active")

        await _engine.rollback_transaction(self._tx_id)
        self._rolled_back = True

    async def savepoint(self, name: str) -> "Savepoint":
        """
        Create a savepoint within this transaction.

        Savepoints allow partial rollback within a transaction.

        Args:
            name: Savepoint name

        Returns:
            Savepoint context manager

        Example:
            >>> async with pg_transaction() as tx:
            ...     await user.save()
            ...     async with tx.savepoint("before_orders"):
            ...         await order1.save()
            ...         await order2.save()
            ...         # Can rollback to savepoint if needed
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        if not self.is_active:
            raise RuntimeError("Transaction is not active")

        savepoint_id = await _engine.create_savepoint(self._tx_id, name)
        return Savepoint(self._tx_id, savepoint_id, name)


class Savepoint:
    """
    PostgreSQL savepoint within a transaction.

    Savepoints allow partial rollback of a transaction.

    Example:
        >>> async with pg_transaction() as tx:
        ...     await user.save()
        ...     async with tx.savepoint("checkpoint") as sp:
        ...         try:
        ...             await risky_operation()
        ...         except Exception:
        ...             await sp.rollback()
    """

    def __init__(self, tx_id: Any, savepoint_id: Any, name: str):
        """
        Initialize savepoint.

        Args:
            tx_id: Transaction identifier
            savepoint_id: Savepoint identifier from Rust backend
            name: Savepoint name
        """
        self._tx_id = tx_id
        self._savepoint_id = savepoint_id
        self._name = name
        self._released = False

    async def rollback(self) -> None:
        """
        Rollback to this savepoint.

        This undoes all changes made after the savepoint was created.

        Raises:
            RuntimeError: If savepoint already released or engine not available
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        if self._released:
            raise RuntimeError("Savepoint has been released")

        await _engine.rollback_to_savepoint(self._tx_id, self._savepoint_id)

    async def release(self) -> None:
        """
        Release this savepoint.

        This destroys the savepoint but keeps the changes.

        Raises:
            RuntimeError: If savepoint already released or engine not available
        """
        if _engine is None:
            raise RuntimeError(
                "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
            )

        if self._released:
            raise RuntimeError("Savepoint already released")

        await _engine.release_savepoint(self._tx_id, self._savepoint_id)
        self._released = True

    async def __aenter__(self) -> "Savepoint":
        """Enter savepoint context."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        """Exit savepoint context."""
        if exc_type is not None and not self._released:
            # Exception occurred - rollback to savepoint
            await self.rollback()
        elif not self._released:
            # No exception - release savepoint
            await self.release()


@asynccontextmanager
async def pg_transaction(
    *,
    isolation_level: Optional[str] = None,
    read_only: bool = False,
    deferrable: bool = False,
) -> AsyncGenerator[Transaction, None]:
    """
    Create a PostgreSQL transaction context.

    The transaction commits on successful exit from the context manager,
    and rolls back if an exception is raised.

    Args:
        isolation_level: Transaction isolation level:
            - "READ UNCOMMITTED"
            - "READ COMMITTED" (default in PostgreSQL)
            - "REPEATABLE READ"
            - "SERIALIZABLE"
        read_only: Whether this is a read-only transaction
        deferrable: Whether constraint checks can be deferred

    Yields:
        Transaction context

    Raises:
        RuntimeError: If PostgreSQL engine is not available

    Example:
        >>> # Basic usage
        >>> async with pg_transaction() as tx:
        ...     user = User(email="alice@example.com", name="Alice")
        ...     await user.save()
        ...     order = Order(user_id=user.id, amount=100.0)
        ...     await order.save()
        ...     # Commits on exit
        >>>
        >>> # With exception - rolls back
        >>> async with pg_transaction() as tx:
        ...     user = User(email="bob@example.com", name="Bob")
        ...     await user.save()
        ...     raise ValueError("Something went wrong")
        ...     # Transaction is rolled back
        >>>
        >>> # Serializable isolation
        >>> async with pg_transaction(isolation_level="SERIALIZABLE") as tx:
        ...     # Highest isolation level
        ...     await critical_operation()
    """
    if _engine is None:
        raise RuntimeError(
            "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
        )

    # Start transaction
    tx_id = await _engine.begin_transaction(
        isolation_level=isolation_level,
        read_only=read_only,
        deferrable=deferrable,
    )

    tx = Transaction(tx_id)

    try:
        yield tx

        # Commit if not already committed/rolled back
        if tx.is_active:
            await tx.commit()

    except Exception:
        # Rollback on exception if not already committed/rolled back
        if tx.is_active:
            await tx.rollback()
        raise


__all__ = [
    "pg_transaction",
    "Transaction",
    "Savepoint",
]
