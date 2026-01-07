"""
Tests for OpenTelemetry integration.

Tests graceful degradation when OpenTelemetry SDK is not installed,
and proper instrumentation when it is available.
"""

import os
import pytest
from unittest.mock import Mock, patch, MagicMock


def test_import_without_otel():
    """Test that telemetry module can be imported without OpenTelemetry SDK."""
    # This test should always pass since we have graceful degradation
    from data_bridge.postgres import telemetry

    assert telemetry is not None


def test_is_tracing_enabled_without_otel():
    """Test is_tracing_enabled returns False when OpenTelemetry is not available."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import is_tracing_enabled

        assert is_tracing_enabled() is False


def test_is_tracing_enabled_with_env_var():
    """Test is_tracing_enabled respects environment variable."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', True):
        from data_bridge.postgres.telemetry import is_tracing_enabled

        # Test enabled (default)
        with patch.dict(os.environ, {}, clear=True):
            assert is_tracing_enabled() is True

        # Test disabled with "false"
        with patch.dict(os.environ, {'DATA_BRIDGE_TRACING_ENABLED': 'false'}):
            assert is_tracing_enabled() is False

        # Test disabled with "0"
        with patch.dict(os.environ, {'DATA_BRIDGE_TRACING_ENABLED': '0'}):
            assert is_tracing_enabled() is False

        # Test disabled with "no"
        with patch.dict(os.environ, {'DATA_BRIDGE_TRACING_ENABLED': 'no'}):
            assert is_tracing_enabled() is False

        # Test enabled with "true"
        with patch.dict(os.environ, {'DATA_BRIDGE_TRACING_ENABLED': 'true'}):
            assert is_tracing_enabled() is True


def test_get_tracer_without_otel():
    """Test get_tracer returns None when OpenTelemetry is not available."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import get_tracer

        tracer = get_tracer()
        assert tracer is None


def test_get_meter_without_otel():
    """Test get_meter returns None when OpenTelemetry is not available."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import get_meter

        meter = get_meter()
        assert meter is None


def test_create_query_span_without_otel():
    """Test create_query_span works without OpenTelemetry (no-op)."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import create_query_span

        with create_query_span("find", "users", filters_count=2) as span:
            assert span is None


def test_create_session_span_without_otel():
    """Test create_session_span works without OpenTelemetry (no-op)."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import create_session_span

        with create_session_span("flush", pending_count=5) as span:
            assert span is None


def test_create_relationship_span_without_otel():
    """Test create_relationship_span works without OpenTelemetry (no-op)."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import create_relationship_span

        with create_relationship_span("user.posts", strategy="selectin") as span:
            assert span is None


def test_add_exception_without_otel():
    """Test add_exception handles None span gracefully."""
    from data_bridge.postgres.telemetry import add_exception

    # Should not raise error
    add_exception(None, ValueError("test error"))


def test_set_span_result_without_otel():
    """Test set_span_result handles None span gracefully."""
    from data_bridge.postgres.telemetry import set_span_result

    # Should not raise error
    set_span_result(None, count=10, affected_rows=5)


@pytest.mark.asyncio
async def test_instrument_span_decorator_async():
    """Test instrument_span decorator on async function."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import instrument_span

        @instrument_span("test.operation")
        async def async_func(value: int) -> int:
            return value * 2

        result = await async_func(5)
        assert result == 10


def test_instrument_span_decorator_sync():
    """Test instrument_span decorator on sync function."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import instrument_span

        @instrument_span("test.operation")
        def sync_func(value: int) -> int:
            return value * 2

        result = sync_func(5)
        assert result == 10


@pytest.mark.asyncio
async def test_instrument_query_decorator_async():
    """Test instrument_query decorator on async function."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import instrument_query

        @instrument_query("find")
        async def async_find(query: str) -> list:
            return ["result1", "result2"]

        result = await async_find("test query")
        assert result == ["result1", "result2"]


def test_instrument_query_decorator_sync():
    """Test instrument_query decorator on sync function."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import instrument_query

        @instrument_query("find")
        def sync_find(query: str) -> list:
            return ["result1", "result2"]

        result = sync_find("test query")
        assert result == ["result1", "result2"]


@pytest.mark.asyncio
async def test_instrument_session_decorator_async():
    """Test instrument_session decorator on async function."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import instrument_session

        @instrument_session("flush")
        async def async_flush() -> None:
            pass

        await async_flush()


def test_instrument_session_decorator_sync():
    """Test instrument_session decorator on sync function."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import instrument_session

        @instrument_session("flush")
        def sync_flush() -> None:
            pass

        sync_flush()


def test_connection_pool_metrics_without_otel():
    """Test ConnectionPoolMetrics without OpenTelemetry."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import ConnectionPoolMetrics

        metrics = ConnectionPoolMetrics()

        # Should not raise error
        metrics.record_pool_stats(in_use=5, idle=3, max_size=10)


def test_get_connection_pool_metrics():
    """Test get_connection_pool_metrics returns singleton instance."""
    with patch('data_bridge.postgres.telemetry.OTEL_AVAILABLE', False):
        from data_bridge.postgres.telemetry import get_connection_pool_metrics

        metrics1 = get_connection_pool_metrics()
        metrics2 = get_connection_pool_metrics()

        # Should return same instance
        assert metrics1 is metrics2


def test_semantic_conventions():
    """Test semantic convention constants are defined."""
    from data_bridge.postgres.telemetry import SpanAttributes, MetricNames

    # Test SpanAttributes
    assert SpanAttributes.DB_SYSTEM == "db.system"
    assert SpanAttributes.DB_OPERATION_NAME == "db.operation.name"
    assert SpanAttributes.DB_COLLECTION_NAME == "db.collection.name"
    assert SpanAttributes.DB_STATEMENT == "db.statement"

    # Test MetricNames
    assert MetricNames.CONNECTION_POOL_IN_USE == "db.connection.pool.in_use"
    assert MetricNames.CONNECTION_POOL_IDLE == "db.connection.pool.idle"
    assert MetricNames.CONNECTION_POOL_MAX == "db.connection.pool.max"


def test_exports():
    """Test that all expected functions are exported."""
    from data_bridge.postgres import telemetry

    expected_exports = [
        "is_tracing_enabled",
        "get_tracer",
        "get_meter",
        "SpanAttributes",
        "MetricNames",
        "create_query_span",
        "create_session_span",
        "create_relationship_span",
        "add_exception",
        "set_span_result",
        "instrument_span",
        "instrument_query",
        "instrument_session",
        "ConnectionPoolMetrics",
        "get_connection_pool_metrics",
    ]

    for export in expected_exports:
        assert hasattr(telemetry, export), f"Missing export: {export}"


def test_module_imports_from_postgres_package():
    """Test that telemetry functions can be imported from postgres package."""
    from data_bridge.postgres import (
        is_tracing_enabled,
        get_tracer,
        get_meter,
        SpanAttributes,
        MetricNames,
        create_query_span,
        create_session_span,
        create_relationship_span,
        add_exception,
        set_span_result,
        instrument_span,
        instrument_query,
        instrument_session,
        ConnectionPoolMetrics,
        get_connection_pool_metrics,
    )

    # All imports should succeed
    assert is_tracing_enabled is not None
    assert get_tracer is not None
    assert get_meter is not None
    assert SpanAttributes is not None
    assert MetricNames is not None
    assert create_query_span is not None
    assert create_session_span is not None
    assert create_relationship_span is not None
    assert add_exception is not None
    assert set_span_result is not None
    assert instrument_span is not None
    assert instrument_query is not None
    assert instrument_session is not None
    assert ConnectionPoolMetrics is not None
    assert get_connection_pool_metrics is not None


def test_query_builder_imports_telemetry():
    """Test that QueryBuilder imports telemetry functions correctly."""
    # This should not raise ImportError
    from data_bridge.postgres.query import QueryBuilder

    # Verify telemetry functions are imported in the module
    import data_bridge.postgres.query as query_module

    assert hasattr(query_module, 'create_query_span')
    assert hasattr(query_module, 'set_span_result')
    assert hasattr(query_module, 'add_exception')
    assert hasattr(query_module, 'is_tracing_enabled')
