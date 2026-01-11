"""
Unit tests for raw SQL execution.

Tests the execute() function with various SQL query types and parameter binding.
"""
import pytest
from unittest.mock import AsyncMock, patch
from data_bridge.test import expect


class TestExecuteFunction:
    """Test raw SQL execution function."""

    @pytest.mark.asyncio
    async def test_execute_raises_without_engine(self):
        """Test execute raises RuntimeError when engine not available."""
        with patch('data_bridge.postgres.connection._engine', None):
            from data_bridge.postgres import execute

            expect(lambda: await execute("SELECT 1")).to_raise(RuntimeError)

    @pytest.mark.asyncio
    async def test_execute_select_query(self, mock_connection_engine):
        """Test execute with SELECT query returns list of dicts."""
        # Mock the engine to return sample data
        mock_connection_engine.execute = AsyncMock(return_value=[
            {"id": 1, "name": "Alice", "age": 30},
            {"id": 2, "name": "Bob", "age": 25}
        ])

        from data_bridge.postgres import execute

        results = await execute("SELECT * FROM users WHERE age > $1", [20])

        mock_connection_engine.execute.assert_called_once()
        expect(len(results)).to_equal(2)
        expect(results[0]["name"]).to_equal("Alice")
        expect(results[1]["name"]).to_equal("Bob")

    @pytest.mark.asyncio
    async def test_execute_insert_query(self, mock_connection_engine):
        """Test execute with INSERT query returns row count."""
        mock_connection_engine.execute = AsyncMock(return_value=1)

        from data_bridge.postgres import execute

        count = await execute(
            "INSERT INTO users (name, age) VALUES ($1, $2)",
            ["Charlie", 35]
        )

        mock_connection_engine.execute.assert_called_once()
        expect(count).to_equal(1)

    @pytest.mark.asyncio
    async def test_execute_update_query(self, mock_connection_engine):
        """Test execute with UPDATE query returns row count."""
        mock_connection_engine.execute = AsyncMock(return_value=3)

        from data_bridge.postgres import execute

        count = await execute(
            "UPDATE users SET age = $1 WHERE age < $2",
            [18, 20]
        )

        mock_connection_engine.execute.assert_called_once()
        expect(count).to_equal(3)

    @pytest.mark.asyncio
    async def test_execute_delete_query(self, mock_connection_engine):
        """Test execute with DELETE query returns row count."""
        mock_connection_engine.execute = AsyncMock(return_value=2)

        from data_bridge.postgres import execute

        count = await execute("DELETE FROM users WHERE age < $1", [18])

        mock_connection_engine.execute.assert_called_once()
        expect(count).to_equal(2)

    @pytest.mark.asyncio
    async def test_execute_ddl_query(self, mock_connection_engine):
        """Test execute with DDL query returns None."""
        mock_connection_engine.execute = AsyncMock(return_value=None)

        from data_bridge.postgres import execute

        result = await execute("CREATE INDEX idx_users_age ON users(age)")

        mock_connection_engine.execute.assert_called_once()
        expect(result).to_be_none()

    @pytest.mark.asyncio
    async def test_execute_without_params(self, mock_connection_engine):
        """Test execute without parameters."""
        mock_connection_engine.execute = AsyncMock(return_value=[{"count": 10}])

        from data_bridge.postgres import execute

        result = await execute("SELECT COUNT(*) as count FROM users")

        mock_connection_engine.execute.assert_called_once()
        expect(result[0]["count"]).to_equal(10)

    @pytest.mark.asyncio
    async def test_execute_with_multiple_params(self, mock_connection_engine):
        """Test execute with multiple parameters."""
        mock_connection_engine.execute = AsyncMock(return_value=[
            {"id": 1, "name": "Alice"}
        ])

        from data_bridge.postgres import execute

        result = await execute(
            "SELECT * FROM users WHERE age BETWEEN $1 AND $2 ORDER BY name LIMIT $3",
            [25, 35, 10]
        )

        mock_connection_engine.execute.assert_called_once()
        # Check that parameters were passed
        call_args = mock_connection_engine.execute.call_args
        expect(call_args[0][1]).to_equal([25, 35, 10])

    @pytest.mark.asyncio
    async def test_execute_with_none_params(self, mock_connection_engine):
        """Test execute with explicit None parameters."""
        mock_connection_engine.execute = AsyncMock(return_value=1)

        from data_bridge.postgres import execute

        count = await execute(
            "INSERT INTO users (name, age, email) VALUES ($1, $2, $3)",
            ["Dave", 40, None]
        )

        mock_connection_engine.execute.assert_called_once()
        expect(count).to_equal(1)

    @pytest.mark.asyncio
    async def test_execute_with_various_types(self, mock_connection_engine):
        """Test execute with various parameter types."""
        mock_connection_engine.execute = AsyncMock(return_value=1)

        from data_bridge.postgres import execute

        # Test with int, str, float, bool
        count = await execute(
            "INSERT INTO test_table (int_col, str_col, float_col, bool_col) VALUES ($1, $2, $3, $4)",
            [42, "text", 3.14, True]
        )

        mock_connection_engine.execute.assert_called_once()
        expect(count).to_equal(1)


class TestExecuteSecurity:
    """Test SQL injection prevention and security."""

    @pytest.mark.asyncio
    async def test_execute_uses_parameterized_queries(self, mock_connection_engine):
        """Test that parameters are bound, not concatenated."""
        mock_connection_engine.execute = AsyncMock(return_value=[])

        from data_bridge.postgres import execute

        # Even with SQL injection attempt, parameters should be safely bound
        malicious_input = "'; DROP TABLE users; --"
        await execute(
            "SELECT * FROM users WHERE name = $1",
            [malicious_input]
        )

        # The parameter should be passed as-is to the engine
        call_args = mock_connection_engine.execute.call_args
        expect(call_args[0][1][0]).to_equal(malicious_input)


class TestExecuteErrorHandling:
    """Test error handling in execute function."""

    @pytest.mark.asyncio
    async def test_execute_handles_query_errors(self, mock_connection_engine):
        """Test execute handles query execution errors."""
        mock_connection_engine.execute = AsyncMock(
            side_effect=RuntimeError("Query execution failed: syntax error")
        )

        from data_bridge.postgres import execute

        expect(lambda: await execute("INVALID SQL QUERY")).to_raise(RuntimeError)

    @pytest.mark.asyncio
    async def test_execute_empty_sql(self, mock_connection_engine):
        """Test execute with empty SQL string."""
        mock_connection_engine.execute = AsyncMock(
            side_effect=RuntimeError("Query execution failed")
        )

        from data_bridge.postgres import execute

        expect(lambda: await execute("")).to_raise(RuntimeError)
