"""
Unit tests for PostgreSQL schema introspection.

Tests schema introspection functions without requiring a real database.
"""
import pytest
from unittest.mock import AsyncMock


class TestSchemaIntrospection:
    """Test schema introspection functions."""

    @pytest.mark.asyncio
    async def test_list_tables(self, mock_connection_engine):
        """Test list_tables returns table names."""
        from data_bridge.postgres.connection import list_tables

        # Mock the Rust function
        mock_connection_engine.list_tables = AsyncMock(
            return_value=["users", "posts", "comments"]
        )

        result = await list_tables("public")

        assert result == ["users", "posts", "comments"]
        mock_connection_engine.list_tables.assert_called_once_with("public")

    @pytest.mark.asyncio
    async def test_list_tables_default_schema(self, mock_connection_engine):
        """Test list_tables with default schema."""
        from data_bridge.postgres.connection import list_tables

        mock_connection_engine.list_tables = AsyncMock(return_value=["users"])

        result = await list_tables()

        assert result == ["users"]
        mock_connection_engine.list_tables.assert_called_once_with("public")

    @pytest.mark.asyncio
    async def test_table_exists_true(self, mock_connection_engine):
        """Test table_exists returns True for existing table."""
        from data_bridge.postgres.connection import table_exists

        mock_connection_engine.table_exists = AsyncMock(return_value=True)

        result = await table_exists("users", "public")

        assert result is True
        mock_connection_engine.table_exists.assert_called_once_with("users", "public")

    @pytest.mark.asyncio
    async def test_table_exists_false(self, mock_connection_engine):
        """Test table_exists returns False for non-existent table."""
        from data_bridge.postgres.connection import table_exists

        mock_connection_engine.table_exists = AsyncMock(return_value=False)

        result = await table_exists("nonexistent", "public")

        assert result is False
        mock_connection_engine.table_exists.assert_called_once_with(
            "nonexistent", "public"
        )

    @pytest.mark.asyncio
    async def test_get_columns(self, mock_connection_engine):
        """Test get_columns returns column information."""
        from data_bridge.postgres.connection import get_columns

        mock_columns = [
            {
                "name": "id",
                "data_type": "Integer",
                "nullable": False,
                "default": "nextval('users_id_seq'::regclass)",
                "is_primary_key": True,
                "is_unique": False,
            },
            {
                "name": "name",
                "data_type": "Varchar(Some(255))",
                "nullable": False,
                "default": None,
                "is_primary_key": False,
                "is_unique": False,
            },
            {
                "name": "email",
                "data_type": "Varchar(Some(255))",
                "nullable": True,
                "default": None,
                "is_primary_key": False,
                "is_unique": True,
            },
        ]

        mock_connection_engine.get_columns = AsyncMock(return_value=mock_columns)

        result = await get_columns("users", "public")

        assert len(result) == 3
        assert result[0]["name"] == "id"
        assert result[0]["is_primary_key"] is True
        assert result[1]["name"] == "name"
        assert result[2]["is_unique"] is True
        mock_connection_engine.get_columns.assert_called_once_with("users", "public")

    @pytest.mark.asyncio
    async def test_get_indexes(self, mock_connection_engine):
        """Test get_indexes returns index information."""
        from data_bridge.postgres.connection import get_indexes

        mock_indexes = [
            {
                "name": "users_pkey",
                "columns": ["id"],
                "is_unique": True,
                "index_type": "btree",
            },
            {
                "name": "idx_users_email",
                "columns": ["email"],
                "is_unique": True,
                "index_type": "btree",
            },
        ]

        mock_connection_engine.get_indexes = AsyncMock(return_value=mock_indexes)

        result = await get_indexes("users", "public")

        assert len(result) == 2
        assert result[0]["name"] == "users_pkey"
        assert result[0]["is_unique"] is True
        assert result[1]["name"] == "idx_users_email"
        mock_connection_engine.get_indexes.assert_called_once_with("users", "public")

    @pytest.mark.asyncio
    async def test_inspect_table(self, mock_connection_engine):
        """Test inspect_table returns complete table information."""
        from data_bridge.postgres.connection import inspect_table

        mock_table_info = {
            "name": "users",
            "schema": "public",
            "columns": [
                {
                    "name": "id",
                    "data_type": "Integer",
                    "nullable": False,
                    "default": "nextval('users_id_seq'::regclass)",
                    "is_primary_key": True,
                    "is_unique": False,
                }
            ],
            "indexes": [
                {
                    "name": "users_pkey",
                    "columns": ["id"],
                    "is_unique": True,
                    "index_type": "btree",
                }
            ],
            "foreign_keys": [],
        }

        mock_connection_engine.inspect_table = AsyncMock(return_value=mock_table_info)

        result = await inspect_table("users", "public")

        assert result["name"] == "users"
        assert result["schema"] == "public"
        assert len(result["columns"]) == 1
        assert len(result["indexes"]) == 1
        assert len(result["foreign_keys"]) == 0
        mock_connection_engine.inspect_table.assert_called_once_with("users", "public")

    @pytest.mark.asyncio
    async def test_schema_functions_raise_without_engine(self):
        """Test schema functions raise RuntimeError when engine not available."""
        from unittest.mock import patch

        with patch("data_bridge.postgres.connection._engine", None):
            from data_bridge.postgres.connection import (
                list_tables,
                table_exists,
                get_columns,
                get_indexes,
                inspect_table,
            )

            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await list_tables()

            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await table_exists("users")

            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await get_columns("users")

            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await get_indexes("users")

            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await inspect_table("users")
