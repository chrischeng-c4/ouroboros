# PyLoop Phase 5: Middleware & Production Features - Implementation Summary

## Overview

Phase 5 adds production-grade middleware support to PyLoop's HTTP server, enabling CORS handling, request/response logging, compression, and a flexible middleware architecture for custom extensions.

## Completed Features

### 1. Base Middleware Architecture

**File**: `/Users/chris.cheng/chris-project/data-bridge/python/data_bridge/pyloop/__init__.py`

- **BaseMiddleware (Abstract)**: Base class for all middleware
  - `process_request()`: Pre-handler processing, can return early response
  - `process_response()`: Post-handler processing, can modify response

**Key Design**:
```python
class BaseMiddleware(ABC):
    @abstractmethod
    async def process_request(self, request: Dict) -> Optional[Dict]:
        """Return None to continue, or dict for early response."""
        pass

    @abstractmethod
    async def process_response(self, request: Dict, response: Dict) -> Dict:
        """Modify and return response."""
        pass
```

### 2. Built-in Middleware Classes

#### CORSMiddleware
- **Purpose**: Handle Cross-Origin Resource Sharing
- **Features**:
  - Wildcard (`*`) or specific origin allowlist
  - Preflight OPTIONS request handling
  - Configurable methods, headers, credentials
  - Preflight cache control (max-age)
  - Proper Vary header handling

```python
app.add_middleware(CORSMiddleware(
    allow_origins=["https://example.com"],
    allow_methods=["GET", "POST", "PUT", "DELETE"],
    allow_headers=["Content-Type", "Authorization"],
    allow_credentials=True,
    max_age=3600
))
```

#### LoggingMiddleware
- **Purpose**: Request/response logging with timing
- **Features**:
  - Tracks request duration (ms)
  - Optional request/response body logging
  - Colored log levels (INFO/WARNING/ERROR by status)
  - Structured logging with extra fields

```python
app.add_middleware(LoggingMiddleware(
    log_request_body=True,
    log_response_body=False
))
```

#### CompressionMiddleware
- **Purpose**: Response compression (gzip)
- **Features**:
  - Automatic gzip compression
  - Configurable minimum size threshold
  - Accept-Encoding header checking
  - Compression level control (1-9)

```python
app.add_middleware(CompressionMiddleware(
    minimum_size=500,
    compression_level=6
))
```

### 3. App Middleware Integration

**New Methods**:
- `add_middleware(middleware)`: Register middleware
- `_process_middleware_request()`: Chain request processing
- `_process_middleware_response()`: Chain response processing (reverse order)
- `_wrap_handler_with_middleware()`: Unified wrapper for all handlers

**Execution Flow**:
```
Request → Middleware 1 → Middleware 2 → Handler → Middleware 2 → Middleware 1 → Response
          (request)      (request)                (response)      (response)
```

**Key Properties**:
- Middleware executes in order for requests
- Middleware executes in reverse order for responses
- Early responses skip handler but still process response middleware
- Error responses are also processed by middleware

### 4. Examples

**File**: `/Users/chris.cheng/chris-project/data-bridge/examples/pyloop_middleware_example.py`

Demonstrates:
- CORS middleware usage
- Logging middleware usage
- Custom authentication middleware
- Custom rate limiting middleware
- Middleware ordering importance

**Custom Middleware Examples**:

```python
class AuthMiddleware(BaseMiddleware):
    """API key authentication."""
    async def process_request(self, request):
        if not valid_api_key(request):
            return {"status": 401, "body": {"error": "Unauthorized"}}
        return None

class RateLimitMiddleware(BaseMiddleware):
    """Simple rate limiting."""
    async def process_request(self, request):
        if rate_limit_exceeded(request):
            return {"status": 429, "body": {"error": "Too many requests"}}
        return None
```

### 5. Test Coverage

**Unit Tests** (`test_pyloop_middleware.py`): 17 tests
- BaseMiddleware abstract enforcement
- CORSMiddleware creation and configuration
- Origin checking (wildcard and specific)
- LoggingMiddleware defaults and timing
- CompressionMiddleware configuration
- App middleware registration
- Middleware ordering
- CORS preflight handling
- Response header modification

**Integration Tests** (`test_pyloop_middleware_integration.py`): 6 tests
- Request/response processing flow
- Early response handling
- Middleware execution order verification
- CORS integration with handlers
- CORS preflight integration
- Error handling with middleware

**All Tests Pass**: 23/23 ✅

## Architecture Details

### Middleware Chain Execution

**Request Phase** (first to last):
1. CORS checks origin and handles preflight
2. Logging records request start time
3. Rate limiting checks limits
4. Auth validates credentials
5. Handler executes (if no early response)

**Response Phase** (last to first - reverse):
1. Auth adds auth headers
2. Rate limiting adds rate limit headers
3. Logging calculates duration and logs
4. CORS adds CORS headers

### Error Handling Integration

Middleware is fully integrated with error handling:
```python
try:
    early_response = await process_middleware_request(request)
    if early_response:
        return await process_middleware_response(request, early_response)

    response = await handler(request)
    return await process_middleware_response(request, response)
except Exception as e:
    error_response = handle_error(e, request)
    return await process_middleware_response(request, error_response)
```

### CORS Implementation Details

**Preflight Handling** (OPTIONS):
```
OPTIONS /api/data
Origin: https://example.com
Access-Control-Request-Method: POST

→ 204 No Content
  Access-Control-Allow-Origin: https://example.com
  Access-Control-Allow-Methods: GET, POST, PUT, DELETE
  Access-Control-Max-Age: 3600
```

**Regular Request**:
```
GET /api/data
Origin: https://example.com

→ 200 OK
  Access-Control-Allow-Origin: https://example.com
  Vary: Origin
```

## API Documentation

### BaseMiddleware

```python
class BaseMiddleware(ABC):
    """Base class for HTTP middleware."""

    @abstractmethod
    async def process_request(self, request: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """
        Process request before handler.

        Args:
            request: Request dict with keys: method, path, headers, body, query_params

        Returns:
            None to continue to handler, or response dict to return early
        """
        pass

    @abstractmethod
    async def process_response(self, request: Dict[str, Any], response: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process response after handler.

        Args:
            request: Original request dict
            response: Response dict with keys: status, body, headers (optional)

        Returns:
            Modified response dict
        """
        pass
```

### CORSMiddleware

```python
CORSMiddleware(
    allow_origins: List[str] = None,      # Default: ["*"]
    allow_methods: List[str] = None,      # Default: ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]
    allow_headers: List[str] = None,      # Default: ["*"]
    expose_headers: List[str] = None,     # Default: []
    allow_credentials: bool = False,      # Default: False
    max_age: int = 600                    # Default: 600 seconds
)
```

### LoggingMiddleware

```python
LoggingMiddleware(
    logger_instance = None,               # Default: data_bridge.pyloop logger
    log_request_body: bool = False,       # Default: False
    log_response_body: bool = False       # Default: False
)
```

### CompressionMiddleware

```python
CompressionMiddleware(
    minimum_size: int = 500,              # Default: 500 bytes
    compression_level: int = 6            # Default: 6 (1-9)
)
```

### App Methods

```python
app.add_middleware(middleware: BaseMiddleware) -> None
    """Add middleware to the app."""
```

## Usage Examples

### Basic CORS

```python
from data_bridge.pyloop import App, CORSMiddleware

app = App()

# Allow all origins
app.add_middleware(CORSMiddleware(allow_origins=["*"]))

@app.get("/api/data")
async def get_data(request):
    return {"data": [1, 2, 3]}
```

### Production Stack

```python
from data_bridge.pyloop import App, CORSMiddleware, LoggingMiddleware
import logging

app = App(debug=False)

# 1. CORS - handle preflight first
app.add_middleware(CORSMiddleware(
    allow_origins=["https://app.example.com"],
    allow_credentials=True
))

# 2. Logging - log all requests
app.add_middleware(LoggingMiddleware(
    logger_instance=logging.getLogger("myapp"),
    log_request_body=False
))

# 3. Custom auth
app.add_middleware(AuthMiddleware(api_key=os.environ["API_KEY"]))
```

### Custom Middleware

```python
class TimingMiddleware(BaseMiddleware):
    """Add X-Response-Time header."""

    async def process_request(self, request):
        import time
        request["_start"] = time.time()
        return None

    async def process_response(self, request, response):
        import time
        duration = time.time() - request.get("_start", 0)

        if "headers" not in response:
            response["headers"] = {}
        response["headers"]["X-Response-Time"] = f"{duration*1000:.2f}ms"

        return response

app.add_middleware(TimingMiddleware())
```

## Testing

### Run All Middleware Tests

```bash
# Unit tests
python -m pytest tests/test_pyloop_middleware.py -v

# Integration tests
python -m pytest tests/test_pyloop_middleware_integration.py -v

# All PyLoop tests (verify no regression)
python -m pytest tests/test_pyloop*.py -v
```

### Manual Testing with Example

```bash
# Start the example server
python examples/pyloop_middleware_example.py

# In another terminal:

# Test health check (no auth required)
curl http://127.0.0.1:8000/health

# Test protected endpoint (requires auth)
curl http://127.0.0.1:8000/ \
  -H 'Authorization: Bearer secret-api-key-123'

# Test CORS preflight
curl -X OPTIONS http://127.0.0.1:8000/data \
  -H 'Origin: http://localhost:3000' \
  -H 'Access-Control-Request-Method: POST' \
  -v

# Test CORS POST
curl -X POST http://127.0.0.1:8000/data \
  -H 'Origin: http://localhost:3000' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer secret-api-key-123' \
  -d '{"name": "test"}' \
  -v
```

## Performance Considerations

### Middleware Overhead

- **Per-request**: O(n) where n = number of middleware
- **CORS preflight**: Returns early, skips handler
- **Logging**: Minimal overhead (<1ms for timing)
- **Compression**: Only for responses > threshold

### Optimization Tips

1. **Order matters**: Put cheaper checks first (CORS → Logging → Auth)
2. **Early returns**: Use for rate limiting, auth failures
3. **Lazy compression**: Only compress large responses
4. **Structured logging**: Use extra fields, not string formatting

## Security Considerations

### CORS Best Practices

- **Don't use `*` with credentials**: Browsers reject this
- **Validate origins**: Use allowlist, not regex
- **Limit methods**: Only allow needed methods
- **Limit headers**: Don't allow `*` with credentials

### Authentication Middleware

- **Check before expensive operations**: Rate limit before auth, auth before DB
- **Use constant-time comparison**: Prevent timing attacks
- **Log auth failures**: For security monitoring
- **Don't expose details**: Generic "Unauthorized" message

## Future Enhancements

### Planned for Phase 6

1. **Rate Limiting with Redis**: Distributed rate limiting
2. **Metrics Middleware**: Prometheus integration
3. **Caching Middleware**: Response caching with ETags
4. **Request ID Middleware**: Distributed tracing

### Possible Extensions

- **JWT Middleware**: Token validation and refresh
- **Session Middleware**: Cookie-based sessions
- **CSRF Middleware**: Cross-site request forgery protection
- **Content Security Policy**: CSP header injection

## Exported APIs

Added to `__all__`:
- `BaseMiddleware`
- `CORSMiddleware`
- `LoggingMiddleware`
- `CompressionMiddleware`

## Files Modified

1. `/Users/chris.cheng/chris-project/data-bridge/python/data_bridge/pyloop/__init__.py`
   - Added middleware classes (350+ lines)
   - Updated App class with middleware support
   - Updated all route decorators

## Files Created

1. `/Users/chris.cheng/chris-project/data-bridge/examples/pyloop_middleware_example.py`
   - Comprehensive middleware example (185 lines)

2. `/Users/chris.cheng/chris-project/data-bridge/tests/test_pyloop_middleware.py`
   - Unit tests (17 tests, 240 lines)

3. `/Users/chris.cheng/chris-project/data-bridge/tests/test_pyloop_middleware_integration.py`
   - Integration tests (6 tests, 220 lines)

## Summary

Phase 5 successfully implements a flexible, production-ready middleware architecture for PyLoop:

- **3 built-in middleware**: CORS, Logging, Compression
- **Extensible base**: Easy to create custom middleware
- **Full integration**: Works with error handling and routing
- **Well tested**: 23 tests covering all scenarios
- **Documented**: Examples and comprehensive docstrings

The middleware system follows best practices:
- Clean separation of concerns
- Predictable execution order
- Easy to understand and extend
- Production-ready security features

**Status**: Phase 5 COMPLETE ✅

**Next**: Phase 6 - Advanced Features (WebSocket, Server-Sent Events, Background Tasks)
