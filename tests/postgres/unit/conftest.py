"""
Pytest fixtures for PostgreSQL unit tests.

These tests don't require a real database connection - they test
the Python API layer without actually connecting to PostgreSQL.
"""
import pytest
from unittest.mock import MagicMock, AsyncMock, patch
from typing import Dict, List, Any


@pytest.fixture
def mock_postgres_engine():
    """
    Mock the Rust engine to avoid actual database calls.

    This fixture patches the _engine module that the postgres package
    imports, allowing unit tests to run without a real database connection.
    """
    with patch('data_bridge.postgres.table._engine') as mock:
        # Mock connection state
        mock.is_connected = MagicMock(return_value=False)

        # Mock connection management
        mock.init = AsyncMock()
        mock.close = AsyncMock()

        # Mock CRUD operations
        mock.insert_one = AsyncMock(return_value=1)
        mock.insert_many = AsyncMock(return_value=[1, 2, 3])
        mock.update_one = AsyncMock(return_value=1)
        mock.update_many = AsyncMock(return_value=5)
        mock.delete_one = AsyncMock(return_value=1)
        mock.delete_many = AsyncMock(return_value=5)

        # Mock query operations
        mock.find_one = AsyncMock(return_value=None)
        mock.find_many = AsyncMock(return_value=[])
        mock.count = AsyncMock(return_value=0)

        yield mock


@pytest.fixture
def sample_table_class():
    """
    Create a sample Table class for testing.

    Returns a User table with common fields for testing.
    """
    from data_bridge.postgres import Table, Column

    class User(Table):
        id: int
        name: str
        email: str
        age: int = 0
        city: str = Column(default="NYC")

        class Settings:
            table_name = "users"
            schema = "public"
            primary_key = "id"

    return User


@pytest.fixture
def sample_user_data() -> Dict[str, Any]:
    """Sample user data for testing."""
    return {
        "id": 1,
        "name": "Test User",
        "email": "test@example.com",
        "age": 25,
        "city": "NYC",
    }


@pytest.fixture
def sample_user_list() -> List[Dict[str, Any]]:
    """Sample list of users for bulk operations testing."""
    return [
        {"name": "Alice", "email": "alice@example.com", "age": 30, "city": "SF"},
        {"name": "Bob", "email": "bob@example.com", "age": 25, "city": "LA"},
        {"name": "Charlie", "email": "charlie@example.com", "age": 35, "city": "NYC"},
    ]


@pytest.fixture
def mock_query_engine():
    """
    Mock the engine for query builder tests.

    Provides mock responses for find_many, count, and other query operations.
    """
    with patch('data_bridge.postgres.query._engine') as mock:
        mock.is_connected = MagicMock(return_value=True)
        mock.find_many = AsyncMock(return_value=[
            {"id": 1, "name": "Alice", "email": "alice@example.com", "age": 30},
            {"id": 2, "name": "Bob", "email": "bob@example.com", "age": 25},
        ])
        mock.count = AsyncMock(return_value=2)

        yield mock


@pytest.fixture
def mock_connection_engine():
    """Mock the engine for connection tests."""
    with patch('data_bridge.postgres.connection._engine') as mock:
        mock.is_connected = MagicMock(return_value=False)
        mock.init = AsyncMock()
        mock.close = AsyncMock()

        yield mock
