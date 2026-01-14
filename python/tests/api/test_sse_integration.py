"""
Integration tests for SSE support with the App framework.

These tests verify that SSE responses work correctly with the data-bridge API.
"""

import asyncio
import pytest
from ouroboros.api import App, ServerSentEvent, EventSourceResponse


@pytest.fixture
def app():
    """Create a test app with SSE endpoints."""
    app = App(title="SSE Test App")

    @app.get("/events/simple")
    async def simple_events():
        async def generate():
            for i in range(3):
                yield ServerSentEvent(data=f"Event {i}")
        return EventSourceResponse(generate())

    @app.get("/events/typed")
    async def typed_events():
        async def generate():
            yield ServerSentEvent(data="Hello", event="greeting", id="1")
            yield ServerSentEvent(data="World", event="greeting", id="2")
        return EventSourceResponse(generate())

    return app


class TestSSEIntegration:
    """Integration tests for SSE with App."""

    @pytest.mark.asyncio
    async def test_sse_endpoint_returns_correct_response_type(self, app):
        """Test that SSE endpoint returns EventSourceResponse."""
        # Find the handler for our route
        handler = None
        for route in app._routes:
            if route.method == "GET" and route.path == "/events/simple":
                handler = route.handler
                break

        assert handler is not None, "Handler not found"

        # Call the handler
        response = await handler()

        # Verify it's an EventSourceResponse
        assert isinstance(response, EventSourceResponse)
        assert response.status_code == 200
        assert response.headers["Content-Type"] == "text/event-stream"

    @pytest.mark.asyncio
    async def test_sse_stream_content(self, app):
        """Test that we can iterate over SSE stream content."""
        # Find the handler
        handler = None
        for route in app._routes:
            if route.method == "GET" and route.path == "/events/simple":
                handler = route.handler
                break

        response = await handler()

        # Collect all events
        events = []
        async for event_bytes in response:
            events.append(event_bytes)

        # Verify we got all events
        assert len(events) == 3
        assert events[0] == b"data: Event 0\n\n"
        assert events[1] == b"data: Event 1\n\n"
        assert events[2] == b"data: Event 2\n\n"

    @pytest.mark.asyncio
    async def test_sse_typed_events(self, app):
        """Test SSE with event types and IDs."""
        # Find the handler
        handler = None
        for route in app._routes:
            if route.method == "GET" and route.path == "/events/typed":
                handler = route.handler
                break

        response = await handler()

        # Collect all events
        events = []
        async for event_bytes in response:
            events.append(event_bytes)

        # Verify event structure
        assert len(events) == 2

        # First event
        assert b"event: greeting\n" in events[0]
        assert b"id: 1\n" in events[0]
        assert b"data: Hello\n" in events[0]

        # Second event
        assert b"event: greeting\n" in events[1]
        assert b"id: 2\n" in events[1]
        assert b"data: World\n" in events[1]

    @pytest.mark.asyncio
    async def test_sse_with_delay(self):
        """Test SSE with time delays between events."""
        async def delayed_generator():
            for i in range(3):
                await asyncio.sleep(0.01)
                yield ServerSentEvent(data=f"Delayed {i}")

        response = EventSourceResponse(delayed_generator())

        start = asyncio.get_event_loop().time()
        events = []
        async for event_bytes in response:
            events.append(event_bytes)
        elapsed = asyncio.get_event_loop().time() - start

        # Should have taken at least 0.03 seconds (3 * 0.01)
        assert elapsed >= 0.025  # Small margin for timing
        assert len(events) == 3

    @pytest.mark.asyncio
    async def test_sse_response_properties(self):
        """Test EventSourceResponse properties."""
        async def dummy_gen():
            yield ServerSentEvent(data="test")

        response = EventSourceResponse(dummy_gen())

        # Check basic properties
        assert response.status_code == 200
        assert response.media_type == "text/event-stream"
        assert response.headers["Content-Type"] == "text/event-stream"
        assert response.headers["Cache-Control"] == "no-cache"
        assert response.headers["Connection"] == "keep-alive"
        assert response.body_bytes() == b""

    @pytest.mark.asyncio
    async def test_sse_custom_headers(self):
        """Test EventSourceResponse with custom headers."""
        async def dummy_gen():
            yield ServerSentEvent(data="test")

        custom_headers = {
            "X-Custom": "value",
            "Access-Control-Allow-Origin": "*"
        }
        response = EventSourceResponse(dummy_gen(), headers=custom_headers)

        assert response.headers["X-Custom"] == "value"
        assert response.headers["Access-Control-Allow-Origin"] == "*"
        assert response.headers["Content-Type"] == "text/event-stream"

    @pytest.mark.asyncio
    async def test_multiple_parallel_streams(self):
        """Test multiple SSE streams can run in parallel."""
        async def generator(prefix: str):
            for i in range(3):
                await asyncio.sleep(0.01)
                yield ServerSentEvent(data=f"{prefix}-{i}")

        response1 = EventSourceResponse(generator("stream1"))
        response2 = EventSourceResponse(generator("stream2"))

        # Collect from both streams in parallel
        async def collect(response):
            events = []
            async for event in response:
                events.append(event)
            return events

        results = await asyncio.gather(collect(response1), collect(response2))

        # Verify both streams completed
        assert len(results[0]) == 3
        assert len(results[1]) == 3
        assert b"stream1-0" in results[0][0]
        assert b"stream2-0" in results[1][0]


if __name__ == "__main__":
    # Run with: uv run python tests/api/test_sse_integration.py
    pytest.main([__file__, "-v"])
