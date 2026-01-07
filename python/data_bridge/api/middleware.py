"""
Middleware system for request/response interception.

Follows the ASGI middleware pattern but simplified for data-bridge.
"""
from typing import Callable, Awaitable, Any, Optional, List
from abc import ABC, abstractmethod
from dataclasses import dataclass
import asyncio

# Type aliases
RequestType = dict  # Simplified request representation
ResponseType = Any
NextMiddleware = Callable[[RequestType], Awaitable[ResponseType]]


class BaseMiddleware(ABC):
    """Base class for middleware implementations."""

    @abstractmethod
    async def __call__(
        self,
        request: RequestType,
        call_next: NextMiddleware
    ) -> ResponseType:
        """
        Process the request and optionally modify response.

        Args:
            request: The incoming request data
            call_next: Call to invoke the next middleware or handler

        Returns:
            The response (possibly modified)
        """
        pass


class MiddlewareStack:
    """Manages the middleware chain."""

    def __init__(self):
        self._middlewares: List[BaseMiddleware] = []

    def add(self, middleware: BaseMiddleware) -> None:
        """Add middleware to the stack (LIFO order - last added runs first)."""
        self._middlewares.insert(0, middleware)

    def wrap(self, handler: Callable) -> Callable:
        """Wrap a handler with all middlewares."""
        async def wrapped(request: RequestType) -> ResponseType:
            # Build the chain from inside out
            async def final_handler(req):
                return await handler(req)

            chain = final_handler
            for middleware in reversed(self._middlewares):
                chain = self._create_next(middleware, chain)

            return await chain(request)

        return wrapped

    def _create_next(
        self,
        middleware: BaseMiddleware,
        next_handler: NextMiddleware
    ) -> NextMiddleware:
        async def next_call(request: RequestType) -> ResponseType:
            return await middleware(request, next_handler)
        return next_call


# Common middleware implementations

class TimingMiddleware(BaseMiddleware):
    """Add request timing to response headers."""

    async def __call__(self, request, call_next):
        import time
        start = time.perf_counter()
        response = await call_next(request)
        duration_ms = (time.perf_counter() - start) * 1000
        # Add timing header if response supports it
        if hasattr(response, 'headers'):
            response.headers["X-Response-Time"] = f"{duration_ms:.2f}ms"
        return response


class LoggingMiddleware(BaseMiddleware):
    """Log requests and responses."""

    def __init__(self, logger=None):
        self.logger = logger

    async def __call__(self, request, call_next):
        # Log request
        if self.logger:
            self.logger.info(f"Request: {request.get('method')} {request.get('path')}")

        response = await call_next(request)

        # Log response
        if self.logger:
            status = getattr(response, 'status_code', 200)
            self.logger.info(f"Response: {status}")

        return response


# CORS Middleware

from typing import Set, Optional
from dataclasses import field


@dataclass
class CORSConfig:
    """CORS configuration."""
    allow_origins: Set[str] = field(default_factory=lambda: {"*"})
    allow_methods: Set[str] = field(default_factory=lambda: {"GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"})
    allow_headers: Set[str] = field(default_factory=lambda: {"*"})
    allow_credentials: bool = False
    expose_headers: Set[str] = field(default_factory=set)
    max_age: int = 600  # 10 minutes


class CORSMiddleware(BaseMiddleware):
    """
    Cross-Origin Resource Sharing middleware.

    Handles preflight OPTIONS requests and adds CORS headers to responses.

    Example:
        app = App()
        app.add_middleware(CORSMiddleware(
            allow_origins={"https://example.com"},
            allow_credentials=True
        ))
    """

    def __init__(
        self,
        allow_origins: Optional[Set[str]] = None,
        allow_methods: Optional[Set[str]] = None,
        allow_headers: Optional[Set[str]] = None,
        allow_credentials: bool = False,
        expose_headers: Optional[Set[str]] = None,
        max_age: int = 600,
    ):
        self.config = CORSConfig(
            allow_origins=allow_origins or {"*"},
            allow_methods=allow_methods or {"GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"},
            allow_headers=allow_headers or {"*"},
            allow_credentials=allow_credentials,
            expose_headers=expose_headers or set(),
            max_age=max_age,
        )

    def _is_origin_allowed(self, origin: str) -> bool:
        """Check if the origin is allowed."""
        if "*" in self.config.allow_origins:
            return True
        return origin in self.config.allow_origins

    def _get_cors_headers(self, origin: str) -> dict:
        """Get CORS headers for regular requests."""
        headers = {}

        # Allow origin
        if self._is_origin_allowed(origin):
            if "*" in self.config.allow_origins and not self.config.allow_credentials:
                headers["Access-Control-Allow-Origin"] = "*"
            else:
                headers["Access-Control-Allow-Origin"] = origin

        # Allow credentials
        if self.config.allow_credentials:
            headers["Access-Control-Allow-Credentials"] = "true"

        # Expose headers
        if self.config.expose_headers:
            headers["Access-Control-Expose-Headers"] = ", ".join(self.config.expose_headers)

        return headers

    def _get_preflight_headers(self, origin: str) -> dict:
        """Get CORS headers for preflight OPTIONS requests."""
        headers = self._get_cors_headers(origin)

        # Allow methods
        headers["Access-Control-Allow-Methods"] = ", ".join(self.config.allow_methods)

        # Allow headers
        if "*" in self.config.allow_headers:
            headers["Access-Control-Allow-Headers"] = "*"
        else:
            headers["Access-Control-Allow-Headers"] = ", ".join(self.config.allow_headers)

        # Max age
        headers["Access-Control-Max-Age"] = str(self.config.max_age)

        return headers

    async def __call__(self, request, call_next):
        origin = request.get("headers", {}).get("origin", "")
        method = request.get("method", "GET")

        # Handle preflight OPTIONS request
        if method == "OPTIONS" and origin:
            from .response import Response
            headers = self._get_preflight_headers(origin)
            return Response(content="", status_code=204, headers=headers)

        # Process request
        response = await call_next(request)

        # Add CORS headers to response
        if origin and self._is_origin_allowed(origin):
            cors_headers = self._get_cors_headers(origin)
            if hasattr(response, 'headers'):
                response.headers.update(cors_headers)

        return response
