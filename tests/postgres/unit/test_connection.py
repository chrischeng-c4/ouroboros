"""
Unit tests for PostgreSQL connection management.

Tests connection initialization and management without requiring a real database.
"""
import pytest
from unittest.mock import patch, AsyncMock, MagicMock
from data_bridge.test import expect


class TestConnectionInit:
    """Test connection initialization."""

    @pytest.mark.asyncio
    async def test_init_with_connection_string(self, mock_connection_engine):
        """Test init with full connection string."""
        from data_bridge.postgres import init

        await init("postgres://user:pass@localhost:5432/mydb")

        mock_connection_engine.init.assert_called_once()
        args = mock_connection_engine.init.call_args[0]
        expect("postgres://user:pass@localhost:5432/mydb" in args).to_be_true()

    @pytest.mark.asyncio
    async def test_init_with_parameters(self, mock_connection_engine):
        """Test init with individual parameters."""
        from data_bridge.postgres import init

        await init(
            host="localhost",
            port=5432,
            database="testdb",
            username="user",
            password="pass",
            max_connections=20,
        )

        mock_connection_engine.init.assert_called_once()
        args = mock_connection_engine.init.call_args[0]
        # Should build connection string
        expect("postgres://" in args[0]).to_be_true()
        expect("user:pass" in args[0]).to_be_true()
        expect("localhost:5432" in args[0]).to_be_true()
        expect("testdb" in args[0]).to_be_true()
        # Should pass connection pool params
        expect(20 in args).to_be_true()  # max_connections

    @pytest.mark.asyncio
    async def test_init_defaults(self, mock_connection_engine):
        """Test init with default parameters."""
        from data_bridge.postgres import init

        await init()

        mock_connection_engine.init.assert_called_once()
        args = mock_connection_engine.init.call_args[0]
        # Should use defaults
        expect("localhost:5432" in args[0]).to_be_true()
        expect("postgres" in args[0]).to_be_true()

    @pytest.mark.asyncio
    async def test_init_without_auth(self, mock_connection_engine):
        """Test init without username/password."""
        from data_bridge.postgres import init

        await init(host="localhost", database="mydb")

        args = mock_connection_engine.init.call_args[0]
        # Should not include auth in connection string
        expect("postgres://localhost:5432/mydb" in args[0]).to_be_true()

    @pytest.mark.asyncio
    async def test_init_raises_without_engine(self):
        """Test init raises RuntimeError when engine not available."""
        with patch('data_bridge.postgres.connection._engine', None):
            from data_bridge.postgres import init

            expect(lambda: await init("postgres://localhost/test")).to_raise(RuntimeError)


class TestConnectionClose:
    """Test connection closing."""

    @pytest.mark.asyncio
    async def test_close(self, mock_connection_engine):
        """Test close() calls engine.close()."""
        from data_bridge.postgres import close

        await close()

        mock_connection_engine.close.assert_called_once()

    @pytest.mark.asyncio
    async def test_close_raises_without_engine(self):
        """Test close raises RuntimeError when engine not available."""
        with patch('data_bridge.postgres.connection._engine', None):
            from data_bridge.postgres import close

            expect(lambda: await close()).to_raise(RuntimeError)


class TestConnectionStatus:
    """Test connection status checking."""

    def test_is_connected_true(self, mock_connection_engine):
        """Test is_connected returns True when connected."""
        mock_connection_engine.is_connected.return_value = True

        from data_bridge.postgres import is_connected

        expect(is_connected()).to_be_true()

    def test_is_connected_false(self, mock_connection_engine):
        """Test is_connected returns False when not connected."""
        mock_connection_engine.is_connected.return_value = False

        from data_bridge.postgres import is_connected

        expect(is_connected()).to_be_false()

    def test_is_connected_no_engine(self):
        """Test is_connected returns False when engine not available."""
        with patch('data_bridge.postgres.connection._engine', None):
            from data_bridge.postgres import is_connected

            expect(is_connected()).to_be_false()


class TestConnectionPooling:
    """Test connection pool configuration."""

    @pytest.mark.asyncio
    async def test_min_connections(self, mock_connection_engine):
        """Test min_connections parameter."""
        from data_bridge.postgres import init

        await init("postgres://localhost/test", min_connections=2)

        args = mock_connection_engine.init.call_args[0]
        expect(2 in args).to_be_true()  # min_connections

    @pytest.mark.asyncio
    async def test_max_connections(self, mock_connection_engine):
        """Test max_connections parameter."""
        from data_bridge.postgres import init

        await init("postgres://localhost/test", max_connections=20)

        args = mock_connection_engine.init.call_args[0]
        expect(20 in args).to_be_true()  # max_connections

    @pytest.mark.asyncio
    async def test_pool_defaults(self, mock_connection_engine):
        """Test connection pool defaults."""
        from data_bridge.postgres import init

        await init("postgres://localhost/test")

        args = mock_connection_engine.init.call_args[0]
        # Check defaults: min=1, max=10
        expect(1 in args or 10 in args).to_be_true()


class TestConnectionStringBuilding:
    """Test connection string construction."""

    @pytest.mark.asyncio
    async def test_build_with_all_params(self, mock_connection_engine):
        """Test connection string with all parameters."""
        from data_bridge.postgres import init

        await init(
            host="db.example.com",
            port=5433,
            database="production",
            username="admin",
            password="secret123",
        )

        args = mock_connection_engine.init.call_args[0]
        conn_str = args[0]

        expect("db.example.com" in conn_str).to_be_true()
        expect("5433" in conn_str).to_be_true()
        expect("production" in conn_str).to_be_true()
        expect("admin" in conn_str).to_be_true()
        expect("secret123" in conn_str).to_be_true()

    @pytest.mark.asyncio
    async def test_connection_string_overrides_params(self, mock_connection_engine):
        """Test connection_string parameter overrides individual params."""
        from data_bridge.postgres import init

        # If connection_string is provided, individual params should be ignored
        await init(
            connection_string="postgres://user@host/db",
            host="ignored",
            database="ignored",
        )

        args = mock_connection_engine.init.call_args[0]
        expect(args[0]).to_equal("postgres://user@host/db")
