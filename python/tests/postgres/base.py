"""
Base classes for PostgreSQL tests using ouroboros.test framework.

All tests connect to a real PostgreSQL instance (Homebrew PostgreSQL 17).
"""

import os
from ouroboros.test import TestSuite, test, expect, fixture
from ouroboros.postgres import init, close, execute


# PostgreSQL connection URI for tests
# Default: Homebrew PostgreSQL 17 with unix socket authentication
POSTGRES_TEST_URI = os.getenv(
    "POSTGRES_URI",
    "postgresql://chris.cheng@localhost:5432/postgres"
)


class PostgresSuite(TestSuite):
    """
    Base class for PostgreSQL tests with real database connection.

    This suite automatically:
    - Initializes the PostgreSQL connection pool before tests
    - Cleans up all tables after each test (for isolation)
    - Closes the connection pool after all tests

    Example:
        from tests.postgres.base import PostgresSuite
        from ouroboros.test import test, expect
        from ouroboros.postgres import execute

        class TestUserCRUD(PostgresSuite):
            async def setup_method(self):
                await execute('''
                    CREATE TABLE IF NOT EXISTS users (
                        id SERIAL PRIMARY KEY,
                        name VARCHAR(255)
                    )
                ''')

            @test
            async def test_insert_user(self):
                await execute("INSERT INTO users (name) VALUES ('Alice')")
                rows = await execute("SELECT * FROM users")
                expect(len(rows)).to_equal(1)
    """

    _db_initialized: bool = False

    async def setup_class(self) -> None:
        """Initialize PostgreSQL connection pool once per test class."""
        if not PostgresSuite._db_initialized:
            await init(POSTGRES_TEST_URI, min_connections=2, max_connections=10)
            PostgresSuite._db_initialized = True

    async def teardown_class(self) -> None:
        """Close PostgreSQL connection pool after all tests."""
        if PostgresSuite._db_initialized:
            await close()
            PostgresSuite._db_initialized = False

    async def teardown_method(self) -> None:
        """Clean up all test tables after each test for isolation."""
        try:
            # Get all tables in public schema
            tables = await execute(
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public'"
            )

            # Drop each table (CASCADE to handle dependencies)
            for row in tables:
                await execute(f"DROP TABLE IF EXISTS {row['tablename']} CASCADE")
        except Exception:
            # Ignore errors during cleanup (e.g., if no tables exist)
            pass


# Alias for backwards compatibility
PostgresIntegrationSuite = PostgresSuite


# Fixture functions for ob-test fixture system

@fixture
async def test_table() -> str:
    """
    Create a standard test table.

    Returns:
        str: The table name ('test_users')
    """
    await execute("""
        CREATE TABLE IF NOT EXISTS test_users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) UNIQUE,
            age INTEGER,
            active BOOLEAN DEFAULT true,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)
    return "test_users"


@fixture
async def sample_users(test_table: str) -> list:
    """
    Insert sample user data into test table.

    Args:
        test_table: The test table name (from test_table fixture)

    Returns:
        list: List of inserted user dictionaries
    """
    users = [
        {"name": "Alice", "email": "alice@example.com", "age": 30},
        {"name": "Bob", "email": "bob@example.com", "age": 25},
        {"name": "Charlie", "email": "charlie@example.com", "age": 35},
    ]

    for user in users:
        await execute(
            f"INSERT INTO {test_table} (name, email, age) VALUES ($1, $2, $3)",
            [user["name"], user["email"], user["age"]]
        )

    return users
