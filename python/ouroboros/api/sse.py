"""
Server-Sent Events (SSE) support for data-bridge API.

This module provides classes for streaming Server-Sent Events to clients.

Example:
    from ouroboros.api import App, ServerSentEvent, EventSourceResponse

    app = App()

    @app.get("/events")
    async def events():
        async def generate():
            for i in range(10):
                yield ServerSentEvent(data=f"Event {i}", event="counter", id=str(i))
        return EventSourceResponse(generate())

    @app.get("/stream")
    async def stream():
        async def generate():
            # Simple data-only events
            yield ServerSentEvent(data="Hello")
            yield ServerSentEvent(data="World")

            # Event with type and ID
            yield ServerSentEvent(
                data="Custom event",
                event="custom",
                id="123"
            )

            # Set retry interval
            yield ServerSentEvent(
                data="With retry",
                retry=5000  # 5 seconds
            )
        return EventSourceResponse(generate())
"""

from typing import AsyncIterator, Dict, Optional
from dataclasses import dataclass

from .response import Response


@dataclass
class ServerSentEvent:
    """Represents a single Server-Sent Event.

    Attributes:
        data: The event data (required). Newlines are automatically handled.
        event: Optional event type. If not set, client receives unnamed events.
        id: Optional event ID. Client can use this for reconnection with Last-Event-ID.
        retry: Optional retry interval in milliseconds for client reconnection.

    Example:
        >>> event = ServerSentEvent(data="Hello, World!")
        >>> event.encode()
        b'data: Hello, World!\\n\\n'

        >>> event = ServerSentEvent(
        ...     data="Status update",
        ...     event="status",
        ...     id="123",
        ...     retry=5000
        ... )
        >>> event.encode()
        b'event: status\\nid: 123\\nretry: 5000\\ndata: Status update\\n\\n'
    """

    data: str
    event: Optional[str] = None
    id: Optional[str] = None
    retry: Optional[int] = None

    def encode(self) -> bytes:
        """Format the event as SSE wire format.

        Returns:
            Encoded event as bytes following the SSE specification.

        Notes:
            - Each field is on its own line with "field: value" format
            - Data can span multiple lines (each line prefixed with "data: ")
            - Events end with double newline (\\n\\n)
        """
        lines = []

        # Add optional fields first (order matters for some clients)
        if self.event is not None:
            lines.append(f"event: {self.event}")

        if self.id is not None:
            lines.append(f"id: {self.id}")

        if self.retry is not None:
            lines.append(f"retry: {self.retry}")

        # Handle multi-line data
        # SSE spec requires each line to be prefixed with "data: "
        # If data is empty or has no newlines, we still need at least one "data: " line
        data_lines = self.data.splitlines()
        if not data_lines:
            # Handle empty string or string with only whitespace
            lines.append(f"data: {self.data}")
        else:
            for line in data_lines:
                lines.append(f"data: {line}")

        # End with double newline
        lines.append("")
        lines.append("")

        return "\n".join(lines).encode("utf-8")


class EventSourceResponse(Response):
    """SSE streaming response.

    This response class streams Server-Sent Events to the client.
    It automatically sets the correct headers for SSE:
    - Content-Type: text/event-stream
    - Cache-Control: no-cache
    - Connection: keep-alive

    Args:
        content: An async iterator that yields ServerSentEvent objects.
        status_code: HTTP status code (default: 200)
        headers: Additional headers to include

    Example:
        >>> async def event_generator():
        ...     for i in range(5):
        ...         await asyncio.sleep(1)
        ...         yield ServerSentEvent(data=f"Count: {i}", event="count")
        >>> response = EventSourceResponse(event_generator())

        >>> # Can also use async generator expression
        >>> response = EventSourceResponse(
        ...     ServerSentEvent(data=str(i)) for i in range(10)
        ... )
    """

    def __init__(
        self,
        content: AsyncIterator[ServerSentEvent],
        status_code: int = 200,
        headers: Optional[Dict[str, str]] = None,
    ):
        # Initialize with SSE-specific headers
        sse_headers = {
            "Content-Type": "text/event-stream",
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",  # Disable nginx buffering
        }

        # Merge user headers (user headers can override defaults)
        if headers:
            sse_headers.update(headers)

        # Store the async iterator before calling super().__init__
        self._event_iterator = content

        super().__init__(
            content=content,
            status_code=status_code,
            headers=sse_headers,
            media_type="text/event-stream",
        )

    def body_bytes(self) -> bytes:
        """Not applicable for streaming responses.

        Returns:
            Empty bytes. The actual streaming is handled by the async iterator.

        Notes:
            SSE responses are streamed event-by-event, not as a single body.
            The framework should check for async iterator protocol instead of
            calling this method.
        """
        return b""

    def __aiter__(self):
        """Return self as the async iterator for streaming events.

        This allows the response object itself to be used in async for loops.
        """
        return self

    async def __anext__(self) -> bytes:
        """Get the next event as encoded bytes.

        Returns:
            Encoded event bytes ready to send to client.

        Raises:
            StopAsyncIteration: When no more events are available.
        """
        event = await self._event_iterator.__anext__()
        return event.encode()
