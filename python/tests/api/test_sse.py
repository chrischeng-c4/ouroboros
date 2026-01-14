"""
Tests for Server-Sent Events (SSE) support.
"""

import asyncio
import pytest
from data_bridge.api import ServerSentEvent, EventSourceResponse


class TestServerSentEvent:
    """Test ServerSentEvent encoding."""

    def test_simple_event(self):
        """Test encoding a simple data-only event."""
        event = ServerSentEvent(data="Hello, World!")
        encoded = event.encode()
        assert encoded == b"data: Hello, World!\n\n"

    def test_event_with_type(self):
        """Test event with type field."""
        event = ServerSentEvent(data="Test data", event="test")
        encoded = event.encode()
        assert b"event: test\n" in encoded
        assert b"data: Test data\n" in encoded
        assert encoded.endswith(b"\n\n")

    def test_event_with_id(self):
        """Test event with ID field."""
        event = ServerSentEvent(data="Test data", id="123")
        encoded = event.encode()
        assert b"id: 123\n" in encoded
        assert b"data: Test data\n" in encoded

    def test_event_with_retry(self):
        """Test event with retry field."""
        event = ServerSentEvent(data="Test data", retry=5000)
        encoded = event.encode()
        assert b"retry: 5000\n" in encoded
        assert b"data: Test data\n" in encoded

    def test_event_with_all_fields(self):
        """Test event with all fields."""
        event = ServerSentEvent(
            data="Complete event",
            event="status",
            id="456",
            retry=3000
        )
        encoded = event.encode()
        assert b"event: status\n" in encoded
        assert b"id: 456\n" in encoded
        assert b"retry: 3000\n" in encoded
        assert b"data: Complete event\n" in encoded
        assert encoded.endswith(b"\n\n")

    def test_multiline_data(self):
        """Test event with multiline data."""
        event = ServerSentEvent(data="Line 1\nLine 2\nLine 3")
        encoded = event.encode()
        # Each line should be prefixed with "data: "
        assert b"data: Line 1\n" in encoded
        assert b"data: Line 2\n" in encoded
        assert b"data: Line 3\n" in encoded
        assert encoded.endswith(b"\n\n")

    def test_empty_data(self):
        """Test event with empty data."""
        event = ServerSentEvent(data="")
        encoded = event.encode()
        assert encoded == b"data: \n\n"

    def test_field_order(self):
        """Test that fields are in the correct order."""
        event = ServerSentEvent(
            data="Test",
            event="type",
            id="1",
            retry=1000
        )
        encoded = event.encode()
        decoded = encoded.decode('utf-8')

        # Check field order: event, id, retry, data
        lines = decoded.split('\n')
        assert lines[0] == "event: type"
        assert lines[1] == "id: 1"
        assert lines[2] == "retry: 1000"
        assert lines[3] == "data: Test"


class TestEventSourceResponse:
    """Test EventSourceResponse streaming."""

    def test_response_headers(self):
        """Test that response has correct SSE headers."""
        async def dummy_generator():
            yield ServerSentEvent(data="test")
            if False:
                yield  # Make it a generator

        response = EventSourceResponse(dummy_generator())

        assert response.status_code == 200
        assert response.headers["Content-Type"] == "text/event-stream"
        assert response.headers["Cache-Control"] == "no-cache"
        assert response.headers["Connection"] == "keep-alive"
        assert response.headers["X-Accel-Buffering"] == "no"
        assert response.media_type == "text/event-stream"

    def test_custom_status_code(self):
        """Test response with custom status code."""
        async def dummy_generator():
            yield ServerSentEvent(data="test")
            if False:
                yield

        response = EventSourceResponse(dummy_generator(), status_code=201)
        assert response.status_code == 201

    def test_additional_headers(self):
        """Test response with additional headers."""
        async def dummy_generator():
            yield ServerSentEvent(data="test")
            if False:
                yield

        custom_headers = {"X-Custom": "value"}
        response = EventSourceResponse(dummy_generator(), headers=custom_headers)

        assert response.headers["X-Custom"] == "value"
        assert response.headers["Content-Type"] == "text/event-stream"

    def test_header_override(self):
        """Test that user headers can override defaults."""
        async def dummy_generator():
            yield ServerSentEvent(data="test")
            if False:
                yield

        custom_headers = {"Cache-Control": "max-age=3600"}
        response = EventSourceResponse(dummy_generator(), headers=custom_headers)

        assert response.headers["Cache-Control"] == "max-age=3600"

    def test_body_bytes_empty(self):
        """Test that body_bytes returns empty for streaming response."""
        async def dummy_generator():
            yield ServerSentEvent(data="test")
            if False:
                yield

        response = EventSourceResponse(dummy_generator())
        assert response.body_bytes() == b""

    @pytest.mark.asyncio
    async def test_async_iteration(self):
        """Test async iteration over events."""
        async def event_generator():
            for i in range(3):
                yield ServerSentEvent(data=f"Event {i}")

        response = EventSourceResponse(event_generator())

        events = []
        async for event_bytes in response:
            events.append(event_bytes)

        assert len(events) == 3
        assert events[0] == b"data: Event 0\n\n"
        assert events[1] == b"data: Event 1\n\n"
        assert events[2] == b"data: Event 2\n\n"

    @pytest.mark.asyncio
    async def test_async_iteration_with_complex_events(self):
        """Test async iteration with complex events."""
        async def event_generator():
            yield ServerSentEvent(data="Hello", event="greeting", id="1")
            yield ServerSentEvent(data="World", event="greeting", id="2")

        response = EventSourceResponse(event_generator())

        events = []
        async for event_bytes in response:
            events.append(event_bytes)

        assert len(events) == 2
        assert b"event: greeting\n" in events[0]
        assert b"id: 1\n" in events[0]
        assert b"data: Hello\n" in events[0]

    @pytest.mark.asyncio
    async def test_async_iteration_with_delay(self):
        """Test async iteration with time delays."""
        async def event_generator():
            for i in range(3):
                await asyncio.sleep(0.01)  # Small delay
                yield ServerSentEvent(data=f"Delayed event {i}")

        response = EventSourceResponse(event_generator())

        events = []
        async for event_bytes in response:
            events.append(event_bytes)

        assert len(events) == 3

    @pytest.mark.asyncio
    async def test_empty_generator(self):
        """Test async iteration over empty generator."""
        async def empty_generator():
            return
            yield  # Make it a generator

        response = EventSourceResponse(empty_generator())

        events = []
        async for event_bytes in response:
            events.append(event_bytes)

        assert len(events) == 0


if __name__ == "__main__":
    # Run with: uv run python tests/api/test_sse.py
    pytest.main([__file__, "-v"])
