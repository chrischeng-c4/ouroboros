"""
data-bridge-api: High-performance API framework

A Rust-based API framework designed as a FastAPI replacement.

Example:
    from data_bridge.api import App, Path, Query, Body

    app = App(title="My API", version="1.0.0")

    @app.get("/users/{user_id}")
    async def get_user(user_id: Annotated[str, Path()]) -> User:
        return await User.get(user_id)
"""

from .app import App, setup_signal_handlers
from .types import Path, Query, Body, Header, Depends
from .dependencies import Scope
from .response import Response, JSONResponse, HTMLResponse, PlainTextResponse
from .exceptions import HTTPException
from .models import BaseModel, Field
from .context import RequestContext
from .http_integration import HttpClientProvider, create_http_client
from .health import HealthManager, HealthCheck, HealthStatus

__all__ = [
    # Core
    "App",
    "setup_signal_handlers",
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
]
