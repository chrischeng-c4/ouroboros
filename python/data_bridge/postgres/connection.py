"""PostgreSQL connection management."""

from typing import Optional, Literal
from contextlib import asynccontextmanager

# Import from Rust engine when available
try:
    from data_bridge.data_bridge import postgres as _engine
except ImportError:
    _engine = None

# Type alias for isolation levels
IsolationLevel = Literal["read_uncommitted", "read_committed", "repeatable_read", "serializable"]


async def init(
    connection_string: Optional[str] = None,
    *,
    host: str = "localhost",
    port: int = 5432,
    database: str = "postgres",
    username: Optional[str] = None,
    password: Optional[str] = None,
    min_connections: int = 1,
    max_connections: int = 10,
) -> None:
    """
    Initialize PostgreSQL connection pool.

    Args:
        connection_string: Full PostgreSQL connection string (postgres://user:pass@host:port/db)
        host: PostgreSQL server hostname (default: localhost)
        port: PostgreSQL server port (default: 5432)
        database: Database name (default: postgres)
        username: Database username
        password: Database password
        min_connections: Minimum number of connections in pool (default: 1)
        max_connections: Maximum number of connections in pool (default: 10)

    Example:
        >>> # Using connection string
        >>> await init("postgres://user:pass@localhost:5432/mydb")
        >>>
        >>> # Using individual parameters
        >>> await init(
        ...     host="localhost",
        ...     port=5432,
        ...     database="mydb",
        ...     username="user",
        ...     password="pass",
        ...     max_connections=20
        ... )

    Raises:
        RuntimeError: If connection fails or Rust engine is not available
    """
    if _engine is None:
        raise RuntimeError(
            "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
        )

    if connection_string is None:
        # Build connection string from individual parameters
        auth = f"{username}:{password}@" if username else ""
        connection_string = f"postgres://{auth}{host}:{port}/{database}"

    await _engine.init(connection_string, min_connections, max_connections)


async def close() -> None:
    """
    Close the PostgreSQL connection pool.

    This should be called when shutting down your application to ensure
    all connections are properly closed.

    Example:
        >>> await close()

    Raises:
        RuntimeError: If Rust engine is not available
    """
    if _engine is None:
        raise RuntimeError(
            "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
        )

    await _engine.close()


def is_connected() -> bool:
    """
    Check if the PostgreSQL connection pool is active.

    Returns:
        True if connected, False otherwise

    Example:
        >>> if is_connected():
        ...     print("Connected to PostgreSQL")
        ... else:
        ...     print("Not connected")
    """
    if _engine is None:
        return False

    return _engine.is_connected()


async def execute(
    sql: str,
    params: Optional[list] = None
):
    """
    Execute raw SQL query with parameter binding.

    This function provides direct SQL execution for power users who need
    to bypass the ORM. It supports:
    - SELECT queries (returns list of dicts)
    - INSERT/UPDATE/DELETE (returns row count)
    - DDL commands like CREATE, ALTER, DROP (returns None)

    WARNING: This bypasses ORM safety features. Use with caution.
    Always use parameterized queries ($1, $2, etc.) to prevent SQL injection.

    Args:
        sql: SQL query string with $1, $2, etc. placeholders
        params: Optional list of parameters to bind to placeholders

    Returns:
        - List[Dict[str, Any]] for SELECT queries (rows as dicts)
        - int for INSERT/UPDATE/DELETE (number of affected rows)
        - None for DDL commands (CREATE, ALTER, DROP, etc.)

    Examples:
        >>> # SELECT query
        >>> users = await execute("SELECT * FROM users WHERE age > $1", [25])
        >>> print(users)  # [{"id": 1, "name": "Alice", "age": 30}, ...]
        >>>
        >>> # INSERT query
        >>> count = await execute(
        ...     "INSERT INTO users (name, age) VALUES ($1, $2)",
        ...     ["Bob", 28]
        ... )
        >>> print(count)  # 1
        >>>
        >>> # UPDATE query
        >>> count = await execute(
        ...     "UPDATE users SET age = $1 WHERE name = $2",
        ...     [29, "Bob"]
        ... )
        >>> print(count)  # 1
        >>>
        >>> # DELETE query
        >>> count = await execute("DELETE FROM users WHERE age < $1", [18])
        >>> print(count)  # Number of deleted rows
        >>>
        >>> # DDL command
        >>> await execute("CREATE INDEX idx_users_age ON users(age)")
        >>> # Returns None
        >>>
        >>> # Complex query with multiple parameters
        >>> results = await execute(
        ...     "SELECT * FROM users WHERE age BETWEEN $1 AND $2 ORDER BY name LIMIT $3",
        ...     [25, 35, 10]
        ... )

    Security Notes:
        - ALWAYS use parameterized queries ($1, $2, etc.)
        - NEVER concatenate user input into SQL strings
        - Parameters are automatically escaped by the database driver

        Bad (SQL injection risk):
            await execute(f"SELECT * FROM users WHERE name = '{user_input}'")

        Good (safe):
            await execute("SELECT * FROM users WHERE name = $1", [user_input])

    Raises:
        RuntimeError: If PostgreSQL engine is not available or query execution fails
        ValueError: If parameter binding fails
    """
    if _engine is None:
        raise RuntimeError(
            "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
        )

    return await _engine.execute(sql, params)


@asynccontextmanager
async def begin_transaction(isolation_level: Optional[IsolationLevel] = None):
    """
    Begin a transaction with automatic rollback on error.

    Transactions ensure ACID properties:
    - Atomicity: All operations succeed or fail together
    - Consistency: Database remains in a valid state
    - Isolation: Concurrent transactions don't interfere
    - Durability: Committed changes persist

    Args:
        isolation_level: Transaction isolation level (default: "read_committed")
            - "read_uncommitted": Lowest isolation, allows dirty reads
            - "read_committed": PostgreSQL default, prevents dirty reads
            - "repeatable_read": Prevents non-repeatable reads
            - "serializable": Highest isolation, prevents phantom reads

    Yields:
        Transaction object with commit() and rollback() methods

    Example:
        >>> # Basic transaction with auto-commit
        >>> async with begin_transaction() as tx:
        ...     # Perform database operations
        ...     await insert_one("users", {"name": "Alice"})
        ...     # Transaction auto-commits on successful exit
        >>>
        >>> # Explicit commit
        >>> async with begin_transaction() as tx:
        ...     await insert_one("users", {"name": "Bob"})
        ...     await tx.commit()  # Explicit commit
        >>>
        >>> # Transaction with specific isolation level
        >>> async with begin_transaction("serializable") as tx:
        ...     # Operations in serializable isolation
        ...     await update_one("accounts", {"id": 1}, {"balance": 1000})
        >>>
        >>> # Auto-rollback on exception
        >>> try:
        ...     async with begin_transaction() as tx:
        ...         await insert_one("users", {"name": "Charlie"})
        ...         raise ValueError("Something went wrong")
        ...         # Transaction auto-rolls back on exception
        ... except ValueError:
        ...     pass

    Raises:
        RuntimeError: If PostgreSQL engine is not available or connection fails
    """
    if _engine is None:
        raise RuntimeError(
            "PostgreSQL engine not available. Ensure data-bridge was built with PostgreSQL support."
        )

    tx = await _engine.begin_transaction(isolation_level)
    committed = False

    try:
        # Wrap commit to track state
        original_commit = tx.commit

        async def tracked_commit():
            nonlocal committed
            await original_commit()
            committed = True

        tx.commit = tracked_commit
        yield tx
    except Exception:
        # Rollback on exception
        if not committed:
            await tx.rollback()
        raise
    else:
        # Auto-commit if not explicitly committed or rolled back
        if not committed:
            await tx.commit()
