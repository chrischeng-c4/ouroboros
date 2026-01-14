"""
data-bridge-api: High-performance API framework

A Rust-based API framework designed as a FastAPI replacement.

Example:
    from ouroboros.api import App, Path, Query, Body

    app = App(title="My API", version="1.0.0")

    @app.get("/users/{user_id}")
    async def get_user(user_id: Annotated[str, Path()]) -> User:
        return await User.get(user_id)
"""

from .app import App, setup_signal_handlers, AppState
from .types import Path, Query, Body, Header, Depends
from .dependencies import Scope
from .response import Response, JSONResponse, HTMLResponse, PlainTextResponse
from .exceptions import HTTPException
from .models import BaseModel, Field
from .context import RequestContext
from .http_integration import HttpClientProvider, create_http_client
from .health import HealthManager, HealthCheck, HealthStatus
from .middleware import BaseMiddleware, MiddlewareStack, TimingMiddleware, LoggingMiddleware, CORSMiddleware, CORSConfig
from .background import BackgroundTasks, get_background_tasks
from .forms import Form, File, UploadFile, FormMarker, FileMarker
from .websocket import WebSocket, WebSocketDisconnect, WebSocketState
from .sse import ServerSentEvent, EventSourceResponse

# Import Rust validation function from the native module
try:
    from ouroboros.ouroboros.api import validate_value as _rust_validate_value
    validate_value = _rust_validate_value
except ImportError:
    # Fallback placeholder if Rust module not available
    def validate_value(data: dict, type_descriptor: dict) -> dict:  # type: ignore
        """Placeholder for Rust validation - returns data unvalidated."""
        return data

__all__ = [
    # Core
    "App",
    "setup_signal_handlers",
    "AppState",
    # Types
    "Path",
    "Query",
    "Body",
    "Header",
    "Depends",
    "Scope",
    # Models
    "BaseModel",
    "Field",
    # Response
    "Response",
    "JSONResponse",
    "HTMLResponse",
    "PlainTextResponse",
    # Exceptions
    "HTTPException",
    # Context
    "RequestContext",
    # HTTP Integration
    "HttpClientProvider",
    "create_http_client",
    # Health
    "HealthManager",
    "HealthCheck",
    "HealthStatus",
    # Middleware
    "BaseMiddleware",
    "MiddlewareStack",
    "TimingMiddleware",
    "LoggingMiddleware",
    "CORSMiddleware",
    "CORSConfig",
    # Background Tasks
    "BackgroundTasks",
    "get_background_tasks",
    # Forms and File Uploads
    "Form",
    "File",
    "UploadFile",
    "FormMarker",
    "FileMarker",
    # WebSocket
    "WebSocket",
    "WebSocketDisconnect",
    "WebSocketState",
    # Server-Sent Events
    "ServerSentEvent",
    "EventSourceResponse",
    # Validation
    "validate_value",
]
