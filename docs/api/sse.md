# Server-Sent Events (SSE) Support

The data-bridge API framework provides built-in support for Server-Sent Events (SSE), enabling real-time server-to-client streaming.

## Overview

Server-Sent Events is a standard for pushing real-time updates from server to client over HTTP. Unlike WebSockets, SSE:
- Uses standard HTTP (no protocol upgrade)
- Is unidirectional (server to client only)
- Automatically handles reconnection
- Works through most proxies and firewalls
- Has built-in event ID and type support

## Quick Start

```python
from data_bridge.api import App, ServerSentEvent, EventSourceResponse

app = App()

@app.get("/events")
async def stream_events():
    async def generate():
        for i in range(10):
            yield ServerSentEvent(data=f"Event {i}")

    return EventSourceResponse(generate())
```

## ServerSentEvent

The `ServerSentEvent` class represents a single event to be sent to the client.

### Constructor

```python
ServerSentEvent(
    data: str,
    event: Optional[str] = None,
    id: Optional[str] = None,
    retry: Optional[int] = None
)
```

**Parameters:**
- `data` (str, required): The event data. Can contain newlines.
- `event` (str, optional): Event type. Clients can listen for specific types.
- `id` (str, optional): Event ID. Used for reconnection tracking.
- `retry` (int, optional): Retry interval in milliseconds for client reconnection.

### Examples

**Simple event:**
```python
event = ServerSentEvent(data="Hello, World!")
# Output: data: Hello, World!\n\n
```

**Event with type and ID:**
```python
event = ServerSentEvent(
    data="Status update",
    event="status",
    id="123"
)
# Output:
# event: status
# id: 123
# data: Status update
#
```

**Multiline data:**
```python
event = ServerSentEvent(data="Line 1\nLine 2\nLine 3")
# Output:
# data: Line 1
# data: Line 2
# data: Line 3
#
```

**With retry interval:**
```python
event = ServerSentEvent(
    data="Important update",
    retry=5000  # Client retries after 5 seconds
)
```

## EventSourceResponse

The `EventSourceResponse` class creates an SSE streaming response.

### Constructor

```python
EventSourceResponse(
    content: AsyncIterator[ServerSentEvent],
    status_code: int = 200,
    headers: Dict[str, str] = None
)
```

**Parameters:**
- `content`: An async iterator that yields `ServerSentEvent` objects
- `status_code`: HTTP status code (default: 200)
- `headers`: Additional HTTP headers

### Automatic Headers

The following headers are set automatically:
- `Content-Type: text/event-stream`
- `Cache-Control: no-cache`
- `Connection: keep-alive`
- `X-Accel-Buffering: no` (disables nginx buffering)

You can override these by passing custom headers.

### Examples

**Basic streaming:**
```python
@app.get("/stream")
async def stream():
    async def generate():
        for i in range(100):
            await asyncio.sleep(1)
            yield ServerSentEvent(data=f"Count: {i}")

    return EventSourceResponse(generate())
```

**With event types:**
```python
@app.get("/notifications")
async def notifications():
    async def generate():
        while True:
            # Get notification from queue
            notification = await notification_queue.get()

            yield ServerSentEvent(
                data=notification.message,
                event=notification.type,  # "info", "warning", "error"
                id=notification.id
            )

    return EventSourceResponse(generate())
```

**Custom headers:**
```python
@app.get("/events")
async def events():
    async def generate():
        yield ServerSentEvent(data="test")

    return EventSourceResponse(
        generate(),
        headers={
            "Access-Control-Allow-Origin": "*",
            "X-Custom-Header": "value"
        }
    )
```

## Client-Side Usage

### JavaScript (Browser)

```javascript
// Connect to event stream
const eventSource = new EventSource('/events');

// Listen for all events
eventSource.onmessage = (event) => {
    console.log('Received:', event.data);
};

// Listen for specific event types
eventSource.addEventListener('status', (event) => {
    console.log('Status:', event.data);
    console.log('Event ID:', event.lastEventId);
});

// Handle errors
eventSource.onerror = (error) => {
    console.error('Error:', error);
    eventSource.close();
};

// Close connection
eventSource.close();
```

### Curl (Testing)

```bash
curl http://localhost:8000/events
```

### Python Client

```python
import httpx

async with httpx.AsyncClient() as client:
    async with client.stream('GET', 'http://localhost:8000/events') as response:
        async for line in response.aiter_lines():
            if line.startswith('data:'):
                data = line[5:].strip()
                print(f"Received: {data}")
```

## Common Patterns

### Real-time Counter

```python
@app.get("/counter")
async def counter():
    async def generate():
        for i in range(100):
            await asyncio.sleep(1)
            yield ServerSentEvent(
                data=str(i),
                event="count",
                id=str(i)
            )

    return EventSourceResponse(generate())
```

### Progress Updates

```python
@app.get("/progress/{task_id}")
async def task_progress(task_id: str):
    async def generate():
        task = get_task(task_id)

        while not task.is_complete():
            progress = task.get_progress()

            yield ServerSentEvent(
                data=json.dumps({
                    "percent": progress.percent,
                    "message": progress.message
                }),
                event="progress",
                id=str(progress.step)
            )

            await asyncio.sleep(0.5)

        # Send completion event
        yield ServerSentEvent(
            data=json.dumps({"status": "complete"}),
            event="done"
        )

    return EventSourceResponse(generate())
```

### Log Streaming

```python
@app.get("/logs/{service}")
async def stream_logs(service: str):
    async def generate():
        async for log_line in log_tailer.follow(service):
            yield ServerSentEvent(
                data=log_line,
                event="log",
                id=str(int(time.time() * 1000))
            )

    return EventSourceResponse(generate())
```

### Live Database Updates

```python
from data_bridge import Document

class Order(Document):
    status: str
    total: float

    class Settings:
        collection = "orders"

@app.get("/orders/live")
async def live_orders():
    async def generate():
        # Watch for changes in MongoDB
        async for change in Order.watch():
            yield ServerSentEvent(
                data=json.dumps({
                    "operation": change.operation,
                    "order_id": str(change.document_id),
                    "data": change.document
                }),
                event="order_update",
                id=str(change.timestamp)
            )

    return EventSourceResponse(generate())
```

### Heartbeat/Keep-Alive

```python
@app.get("/events")
async def events_with_heartbeat():
    async def generate():
        while True:
            # Send actual events
            if has_event():
                event = get_event()
                yield ServerSentEvent(
                    data=event.data,
                    event=event.type
                )
            else:
                # Send heartbeat comment (keeps connection alive)
                yield ServerSentEvent(data="", event="heartbeat")

            await asyncio.sleep(10)

    return EventSourceResponse(generate())
```

## Best Practices

### 1. Error Handling

Always handle errors in your generator:

```python
@app.get("/events")
async def events():
    async def generate():
        try:
            while True:
                data = await get_data()
                yield ServerSentEvent(data=data)
        except Exception as e:
            # Send error event
            yield ServerSentEvent(
                data=json.dumps({"error": str(e)}),
                event="error"
            )
            # Generator will stop after this

    return EventSourceResponse(generate())
```

### 2. Client Disconnection

The framework handles client disconnection automatically. Your generator will be cancelled when the client disconnects.

```python
@app.get("/events")
async def events():
    async def generate():
        try:
            while True:
                yield ServerSentEvent(data="ping")
                await asyncio.sleep(1)
        except asyncio.CancelledError:
            # Clean up resources
            await cleanup()
            raise  # Re-raise to properly cancel

    return EventSourceResponse(generate())
```

### 3. Resource Management

Use context managers for proper cleanup:

```python
@app.get("/events")
async def events():
    async def generate():
        async with database.connection() as conn:
            async for row in conn.stream_query("SELECT * FROM events"):
                yield ServerSentEvent(data=json.dumps(row))

    return EventSourceResponse(generate())
```

### 4. Buffering

Disable buffering in nginx if you're behind a proxy:

```nginx
location /events {
    proxy_pass http://backend;
    proxy_buffering off;
    proxy_cache off;
    proxy_set_header Connection '';
    proxy_http_version 1.1;
    chunked_transfer_encoding off;
}
```

### 5. Reconnection Strategy

Set appropriate retry intervals:

```python
yield ServerSentEvent(
    data="data",
    retry=5000  # Retry after 5 seconds
)
```

### 6. Event IDs for Resumption

Use event IDs to support reconnection from last received event:

```python
@app.get("/events")
async def events(last_event_id: str = Header(None, alias="Last-Event-ID")):
    async def generate():
        # Start from last received event
        start_from = int(last_event_id) if last_event_id else 0

        for i in range(start_from, 1000):
            yield ServerSentEvent(
                data=f"Event {i}",
                id=str(i)
            )

    return EventSourceResponse(generate())
```

## Testing

### Unit Testing

```python
import pytest
from data_bridge.api import ServerSentEvent, EventSourceResponse

@pytest.mark.asyncio
async def test_event_stream():
    async def generate():
        for i in range(3):
            yield ServerSentEvent(data=f"Event {i}")

    response = EventSourceResponse(generate())

    events = []
    async for event_bytes in response:
        events.append(event_bytes)

    assert len(events) == 3
    assert events[0] == b"data: Event 0\n\n"
```

### Integration Testing

```python
import httpx
import pytest

@pytest.mark.asyncio
async def test_sse_endpoint(test_client):
    async with test_client.stream('GET', '/events') as response:
        assert response.status_code == 200
        assert response.headers['content-type'] == 'text/event-stream'

        # Read first event
        lines = []
        async for line in response.aiter_lines():
            lines.append(line)
            if line == '':  # Empty line marks end of event
                break

        assert lines[0].startswith('data:')
```

## Performance Considerations

1. **Memory**: Each connection keeps the generator in memory
2. **Connections**: Monitor number of concurrent SSE connections
3. **Timeouts**: Set appropriate timeouts for long-lived connections
4. **Backpressure**: If client can't keep up, events may be dropped

## Comparison with WebSockets

| Feature | SSE | WebSocket |
|---------|-----|-----------|
| Direction | Server → Client | Bidirectional |
| Protocol | HTTP | WS/WSS |
| Reconnection | Automatic | Manual |
| Binary data | No | Yes |
| Proxy support | Excellent | Good |
| Complexity | Simple | Complex |

Use SSE when you need:
- Server-to-client streaming only
- Automatic reconnection
- Simple implementation
- Good proxy compatibility

Use WebSocket when you need:
- Bidirectional communication
- Binary data transfer
- Lower latency
- More control over connection

## See Also

- [Response Classes](response.md)
- [WebSocket Support](websocket.md)
- [API Examples](../examples/sse.md)
