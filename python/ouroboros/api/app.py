"""
Main App class for the API framework.
"""

from typing import (
    Any, Callable, Dict, List, Optional, Type, TypeVar, Union,
    get_type_hints, get_origin, get_args, Annotated, AsyncGenerator
)
import inspect
import functools
import asyncio
from contextlib import asynccontextmanager
from dataclasses import dataclass, field

from .types import Path, Query, Body, Header, Depends
from .response import Response, JSONResponse, HTMLResponse
from .exceptions import HTTPException
from .type_extraction import extract_handler_meta
from .dependencies import (
    DependencyContainer, RequestContext,
    extract_dependencies, build_dependency_graph, Scope
)
from .openapi import generate_openapi, get_swagger_ui_html, get_redoc_html
from .http_integration import HttpClientProvider
from .health import HealthManager
from .middleware import MiddlewareStack, BaseMiddleware
from .forms import FormMarker, FileMarker, UploadFile

# Import Rust bindings
try:
    # Import the Rust module (ouroboros is the compiled .so file)
    # It exposes submodules like ouroboros.ouroboros.api
    from ..ouroboros import api as _api
except ImportError:
    _api = None

T = TypeVar('T')

class AppState:
    """Simple state container for lifespan-scoped data.

    Allows storing arbitrary attributes that persist for the application's lifetime.
    This is useful for storing database connections, caches, or other resources
    that should be initialized during startup and cleaned up during shutdown.

    Example:
        @asynccontextmanager
        async def lifespan(app: App):
            # Startup
            app.state.db = await connect_db()
            app.state.cache = Redis()
            yield
            # Shutdown
            await app.state.db.close()
            await app.state.cache.close()
    """
    pass

@dataclass
class RouteInfo:
    """Information about a registered route."""
    method: str
    path: str
    handler: Callable
    name: str
    summary: Optional[str]
    description: Optional[str]
    tags: List[str]
    deprecated: bool
    status_code: int
    dependencies: List[str] = field(default_factory=list)

class App:
    """API Application.

    Example:
        app = App(title="My API", version="1.0.0")

        @app.get("/users/{user_id}")
        async def get_user(user_id: str) -> User:
            return await User.get(user_id)

        # Run the app (for local development)
        app.run(host="0.0.0.0", port=8000)
    """

    def __init__(
        self,
        *,
        title: str = "API",
        version: str = "1.0.0",
        description: str = "",
        docs_url: str = "/docs",
        redoc_url: str = "/redoc",
        openapi_url: str = "/openapi.json",
        shutdown_timeout: float = 30.0,
        lifespan: Optional[Callable[["App"], AsyncGenerator]] = None,
    ):
        self.title = title
        self.version = version
        self.description = description
        self.docs_url = docs_url
        self.redoc_url = redoc_url
        self.openapi_url = openapi_url

        self._routes: List[RouteInfo] = []
        self._handlers: Dict[str, Callable] = {}
        self._websocket_handlers: Dict[str, Callable] = {}
        self._dependency_container = DependencyContainer()
        self._compiled = False
        self._global_deps: Dict[int, str] = {}  # Track factory id -> registered name
        self._docs_setup = False
        self._http_provider = HttpClientProvider()
        self._shutdown_timeout = shutdown_timeout
        self.is_shutting_down = False
        self._middleware_stack = MiddlewareStack()
        self._lifespan = lifespan
        self.state = AppState()

        # Health management
        self._health_manager = HealthManager()
        self._startup_hooks: List[Callable] = []
        self._shutdown_hooks: List[Callable] = []

        # Auto-register health hooks
        self._startup_hooks.append(lambda: self._health_manager.set_ready(True))
        self._shutdown_hooks.append(lambda: self._health_manager.set_ready(False))

        # Initialize Rust app if available
        if _api is not None:
            self._rust_app = _api.ApiApp(title=title, version=version)
        else:
            self._rust_app = None

    def route(
        self,
        path: str,
        *,
        methods: Optional[List[str]] = None,
        name: Optional[str] = None,
        summary: Optional[str] = None,
        description: Optional[str] = None,
        tags: Optional[List[str]] = None,
        deprecated: bool = False,
        status_code: int = 200,
    ) -> Callable[[T], T]:
        """Register a route handler.

        Example:
            @app.route("/users", methods=["GET", "POST"])
            async def users(request: Request) -> Response:
                ...
        """
        methods = methods or ["GET"]
        tags = tags or []

        def decorator(func: T) -> T:
            nonlocal name, summary, description

            name = name or func.__name__
            summary = summary or func.__doc__
            if summary:
                summary = summary.split('\n')[0].strip()

            for method in methods:
                self._register_route(
                    method=method.upper(),
                    path=path,
                    handler=func,
                    name=name,
                    summary=summary,
                    description=description,
                    tags=tags,
                    deprecated=deprecated,
                    status_code=status_code,
                )

            return func

        return decorator

    def get(
        self,
        path: str,
        *,
        name: Optional[str] = None,
        summary: Optional[str] = None,
        description: Optional[str] = None,
        tags: Optional[List[str]] = None,
        deprecated: bool = False,
        status_code: int = 200,
    ) -> Callable[[T], T]:
        """Register a GET route handler."""
        return self.route(
            path,
            methods=["GET"],
            name=name,
            summary=summary,
            description=description,
            tags=tags,
            deprecated=deprecated,
            status_code=status_code,
        )

    def post(
        self,
        path: str,
        *,
        name: Optional[str] = None,
        summary: Optional[str] = None,
        description: Optional[str] = None,
        tags: Optional[List[str]] = None,
        deprecated: bool = False,
        status_code: int = 201,
    ) -> Callable[[T], T]:
        """Register a POST route handler."""
        return self.route(
            path,
            methods=["POST"],
            name=name,
            summary=summary,
            description=description,
            tags=tags,
            deprecated=deprecated,
            status_code=status_code,
        )

    def put(
        self,
        path: str,
        *,
        name: Optional[str] = None,
        summary: Optional[str] = None,
        description: Optional[str] = None,
        tags: Optional[List[str]] = None,
        deprecated: bool = False,
        status_code: int = 200,
    ) -> Callable[[T], T]:
        """Register a PUT route handler."""
        return self.route(
            path,
            methods=["PUT"],
            name=name,
            summary=summary,
            description=description,
            tags=tags,
            deprecated=deprecated,
            status_code=status_code,
        )

    def patch(
        self,
        path: str,
        *,
        name: Optional[str] = None,
        summary: Optional[str] = None,
        description: Optional[str] = None,
        tags: Optional[List[str]] = None,
        deprecated: bool = False,
        status_code: int = 200,
    ) -> Callable[[T], T]:
        """Register a PATCH route handler."""
        return self.route(
            path,
            methods=["PATCH"],
            name=name,
            summary=summary,
            description=description,
            tags=tags,
            deprecated=deprecated,
            status_code=status_code,
        )

    def delete(
        self,
        path: str,
        *,
        name: Optional[str] = None,
        summary: Optional[str] = None,
        description: Optional[str] = None,
        tags: Optional[List[str]] = None,
        deprecated: bool = False,
        status_code: int = 204,
    ) -> Callable[[T], T]:
        """Register a DELETE route handler."""
        return self.route(
            path,
            methods=["DELETE"],
            name=name,
            summary=summary,
            description=description,
            tags=tags,
            deprecated=deprecated,
            status_code=status_code,
        )

    def websocket(
        self,
        path: str,
        *,
        name: Optional[str] = None,
        summary: Optional[str] = None,
        description: Optional[str] = None,
        tags: Optional[List[str]] = None,
        deprecated: bool = False,
    ) -> Callable[[T], T]:
        """Register a WebSocket route handler.

        Example:
            @app.websocket("/ws/{room_id}")
            async def websocket_handler(websocket: WebSocket, room_id: str):
                await websocket.accept()
                while True:
                    data = await websocket.receive_text()
                    await websocket.send_text(f"Echo: {data}")
        """
        tags = tags or []

        def decorator(func: T) -> T:
            nonlocal name, summary, description

            name = name or func.__name__
            summary = summary or func.__doc__
            if summary:
                summary = summary.split('\n')[0].strip()

            # Store WebSocket handler
            self._websocket_handlers[path] = func

            return func

        return decorator

    def _register_route(
        self,
        method: str,
        path: str,
        handler: Callable,
        name: str,
        summary: Optional[str],
        description: Optional[str],
        tags: List[str],
        deprecated: bool,
        status_code: int,
    ) -> None:
        """Internal route registration."""
        # Build dependency graph for handler using global deps tracker
        handler_deps = build_dependency_graph(
            handler,
            self._dependency_container,
            prefix=f"{method}:{path}:",
            _global_deps=self._global_deps,
        )

        route_info = RouteInfo(
            method=method,
            path=path,
            handler=handler,
            name=name,
            summary=summary,
            description=description,
            tags=tags,
            deprecated=deprecated,
            status_code=status_code,
            dependencies=handler_deps,
        )
        self._routes.append(route_info)
        self._handlers[f"{method}:{path}"] = handler

        # Register with Rust app if available
        if self._rust_app is not None:
            # Extract handler metadata for validation
            meta = extract_handler_meta(handler, method, path)

            # Build metadata dict, excluding None values
            metadata_dict = {
                "deprecated": deprecated,
                "status_code": status_code,
            }
            if name is not None:
                metadata_dict["operation_id"] = name
            if summary is not None:
                metadata_dict["summary"] = summary
            if description is not None:
                metadata_dict["description"] = description
            if tags is not None:
                metadata_dict["tags"] = tags

            self._rust_app.register_route(
                method=method,
                path=path,
                handler=handler,
                validator_dict=meta.get("validator"),
                metadata_dict=metadata_dict,
            )

    def openapi(self) -> dict:
        """Generate OpenAPI schema."""
        if self._rust_app is not None:
            import json
            return json.loads(self._rust_app.openapi_json())

        # Fallback: generate in Python
        return generate_openapi(
            title=self.title,
            version=self.version,
            description=self.description,
            routes=self._routes,
        )

    def openapi_json(self) -> str:
        """Get OpenAPI schema as JSON string."""
        import json
        return json.dumps(self.openapi(), indent=2)

    def setup_docs(self) -> None:
        """Setup documentation endpoints."""
        if self._docs_setup:
            return
        self._docs_setup = True

        if self.openapi_url:
            @self.get(self.openapi_url, tags=["documentation"])
            async def openapi_schema():
                """OpenAPI schema."""
                return JSONResponse(self.openapi())

        if self.docs_url:
            @self.get(self.docs_url, tags=["documentation"])
            async def swagger_ui():
                """Swagger UI documentation."""
                return HTMLResponse(
                    get_swagger_ui_html(self.title, self.openapi_url)
                )

        if self.redoc_url:
            @self.get(self.redoc_url, tags=["documentation"])
            async def redoc():
                """ReDoc documentation."""
                return HTMLResponse(
                    get_redoc_html(self.title, self.openapi_url)
                )

    @property
    def routes(self) -> List[RouteInfo]:
        """Get all registered routes."""
        return self._routes.copy()

    def compile(self) -> None:
        """Compile the app (dependencies and routes)."""
        if self._compiled:
            return

        self._dependency_container.compile()
        self._compiled = True

    async def resolve_dependencies(
        self,
        handler: Callable,
        context: Optional[RequestContext] = None,
    ) -> Dict[str, Any]:
        """Resolve dependencies for a handler.

        Args:
            handler: The handler function
            context: Optional request context

        Returns:
            Dictionary mapping parameter names to resolved dependencies
        """
        if not self._compiled:
            self.compile()

        if context is None:
            context = RequestContext()

        deps = extract_dependencies(handler)
        if not deps:
            return {}

        # Find the route info for this handler to get registered dependency names
        route_deps = None
        for route in self._routes:
            if route.handler is handler:
                route_deps = route.dependencies
                break

        if route_deps is None:
            # Handler not registered as a route, register dependencies on-the-fly
            route_deps = build_dependency_graph(
                handler,
                self._dependency_container,
                prefix="",
                _global_deps=self._global_deps,
            )
            self._dependency_container.compile()

        # Resolve using registered names
        resolved = await self._dependency_container.resolve_all(route_deps, context)

        # Map back to parameter names
        result: Dict[str, Any] = {}
        param_to_dep_name = {
            param_name: dep_name
            for param_name, dep_name in zip(deps.keys(), route_deps)
        }

        for param_name, dep_name in param_to_dep_name.items():
            if dep_name in resolved:
                result[param_name] = resolved[dep_name]

        return result

    def configure_http_client(
        self,
        base_url: Optional[str] = None,
        timeout: float = 30.0,
        connect_timeout: float = 10.0,
        **kwargs
    ):
        """Configure the HTTP client for making external requests.

        This makes HttpClient available as a dependency in route handlers.

        Args:
            base_url: Base URL for all requests
            timeout: Request timeout in seconds (default: 30.0)
            connect_timeout: Connection timeout in seconds (default: 10.0)
            **kwargs: Additional HttpClient configuration (pool_max_idle_per_host,
                     follow_redirects, max_redirects, user_agent, etc.)

        Example:
            app = App()
            app.configure_http_client(
                base_url="https://api.example.com",
                timeout=30.0
            )

            @app.get("/data")
            async def get_data(http: HttpClient = Depends()):
                response = await http.get("/users")
                return response.json()
        """
        from ..http import HttpClient

        # Configure the provider
        self._http_provider.configure(
            base_url=base_url,
            timeout=timeout,
            connect_timeout=connect_timeout,
            **kwargs
        )

        # Register HttpClient as singleton dependency
        self._dependency_container.register(
            "HttpClient",
            self._http_provider,
            scope=Scope.SINGLETON
        )

    @property
    def http_client(self):
        """Get the configured HTTP client.

        Returns:
            Configured HttpClient instance

        Raises:
            RuntimeError: If HTTP client not configured yet

        Example:
            app.configure_http_client(base_url="https://api.example.com")
            response = await app.http_client.get("/data")
        """
        return self._http_provider.get_client()

    @property
    def health(self) -> HealthManager:
        """Get the health manager.

        Returns:
            HealthManager instance for registering health checks

        Example:
            app.health.add_check("database", lambda: db.is_connected())
        """
        return self._health_manager

    def include_health_routes(self, prefix: str = "") -> None:
        """Add K8s health check endpoints.

        Args:
            prefix: URL prefix for health endpoints (default: "")

        Registers:
            - GET {prefix}/health - Overall health status
            - GET {prefix}/live - Liveness probe (always 200 if running)
            - GET {prefix}/ready - Readiness probe (503 if not ready)

        Example:
            app.include_health_routes()  # /health, /live, /ready
            app.include_health_routes("/api")  # /api/health, /api/live, /api/ready
        """
        @self.get(f"{prefix}/health", tags=["health"])
        async def health_check() -> dict:
            """Health check endpoint."""
            status = await self._health_manager.check_health()
            return status.to_dict()

        @self.get(f"{prefix}/live", tags=["health"])
        async def liveness() -> dict:
            """Liveness probe endpoint."""
            return {"status": "alive"}

        @self.get(f"{prefix}/ready", tags=["health"])
        async def readiness():
            """Readiness probe endpoint."""
            is_ready = await self._health_manager.is_ready()
            if is_ready:
                return JSONResponse({"status": "ready"}, status_code=200)
            else:
                return JSONResponse({"status": "not_ready"}, status_code=503)

    async def startup(self) -> None:
        """Run all startup hooks.

        Called when the application starts to initialize resources
        and set readiness state.
        """
        for hook in self._startup_hooks:
            if asyncio.iscoroutinefunction(hook):
                await hook()
            else:
                hook()

    def on_startup(self, func: Callable) -> Callable:
        """Register a startup hook.

        Args:
            func: Function to run on startup (can be sync or async)

        Returns:
            The original function (decorator pattern)

        Example:
            @app.on_startup
            async def init_db():
                await db.connect()
        """
        self._startup_hooks.append(func)
        return func

    def on_shutdown(self, func: Callable) -> Callable:
        """Register a shutdown hook.

        Args:
            func: Function to run on shutdown (can be sync or async)

        Returns:
            The original function (decorator pattern)

        Example:
            @app.on_shutdown
            async def close_db():
                await db.close()
        """
        self._shutdown_hooks.append(func)
        return func

    async def shutdown(self, timeout: Optional[float] = None) -> None:
        """Gracefully shutdown the application.

        Closes all resources including HTTP client connections and
        cleans up the dependency container.

        Args:
            timeout: Timeout in seconds for each shutdown hook
                    (default: uses shutdown_timeout from __init__)
        """
        self.is_shutting_down = True

        if timeout is None:
            timeout = self._shutdown_timeout

        # Run shutdown hooks in LIFO (reverse) order
        for hook in reversed(self._shutdown_hooks):
            try:
                if asyncio.iscoroutinefunction(hook):
                    await asyncio.wait_for(hook(), timeout=timeout)
                else:
                    hook()
            except asyncio.TimeoutError:
                pass  # Hook timed out, continue with others
            except Exception:
                pass  # Best effort cleanup

        # Close HTTP client if configured
        try:
            await self._http_provider.close()
        except Exception:
            pass  # Best effort cleanup

        # Additional cleanup can be added here
        # - Close database connections
        # - Flush logs
        # - etc.

    def add_middleware(self, middleware: BaseMiddleware) -> None:
        """Add middleware to the application.

        Middleware will be executed in LIFO order (last added runs first).

        Args:
            middleware: Middleware instance to add to the stack

        Example:
            from ouroboros.api import App, TimingMiddleware, LoggingMiddleware

            app = App()
            app.add_middleware(TimingMiddleware())
            app.add_middleware(LoggingMiddleware())
        """
        self._middleware_stack.add(middleware)

    async def _parse_form_data(self, request: Any) -> Optional[Dict[str, Any]]:
        """Parse form data from request (delegated to Rust).

        This method will be called by the Rust engine when processing
        multipart/form-data or application/x-www-form-urlencoded requests.

        Args:
            request: Request object from Rust engine

        Returns:
            Dictionary containing:
                - fields: Dict[str, str] for text fields
                - files: List[Dict] for uploaded files, each with:
                    - field_name: Form field name
                    - filename: Original filename
                    - content_type: MIME type
                    - data: File contents as bytes

        Note:
            This is a stub implementation. The actual parsing will be
            implemented in the Rust layer for performance.

        Example:
            form_data = await app._parse_form_data(request)
            if form_data:
                name = form_data["fields"].get("name")
                files = form_data["files"]
        """
        # TODO: Call Rust engine to parse multipart/urlencoded
        # For now, return None (will implement Rust binding later)
        return None

    def _resolve_form_parameters(
        self,
        handler: Callable,
        form_fields: Dict[str, str],
        uploaded_files: Dict[str, UploadFile],
    ) -> Dict[str, Any]:
        """Resolve form field and file upload parameters for a handler.

        Inspects the handler signature for Form and File markers and
        extracts the corresponding values from form data.

        Args:
            handler: Handler function to resolve parameters for
            form_fields: Text form fields (name -> value)
            uploaded_files: Uploaded files (field_name -> UploadFile)

        Returns:
            Dictionary mapping parameter names to resolved values

        Example:
            kwargs = app._resolve_form_parameters(
                handler,
                {"name": "John", "email": "john@example.com"},
                {"avatar": UploadFile(...)}
            )
            # kwargs = {"name": "John", "email": "john@example.com", "avatar": UploadFile(...)}
        """
        sig = inspect.signature(handler)
        kwargs: Dict[str, Any] = {}

        for param_name, param in sig.parameters.items():
            # Check for Form marker
            if isinstance(param.default, FormMarker):
                if param_name in form_fields:
                    # Type conversion based on annotation
                    value = form_fields[param_name]
                    if param.annotation != inspect.Parameter.empty:
                        # Try to convert to the annotated type
                        try:
                            # Handle Optional types
                            origin = get_origin(param.annotation)
                            if origin is Union:
                                # Get the non-None type from Optional
                                args = get_args(param.annotation)
                                target_type = next(
                                    (arg for arg in args if arg is not type(None)),
                                    str
                                )
                            else:
                                target_type = param.annotation

                            # Convert string to target type
                            if target_type in (int, float, bool):
                                value = target_type(value)
                        except (ValueError, TypeError):
                            # Keep as string if conversion fails
                            pass
                    kwargs[param_name] = value
                elif param.default.default is not ...:
                    # Use default value
                    kwargs[param_name] = param.default.default
                # else: required field missing, will be caught by handler

            # Check for File marker
            elif isinstance(param.default, FileMarker):
                if param_name in uploaded_files:
                    kwargs[param_name] = uploaded_files[param_name]
                elif param.default.default is not ...:
                    # Use default value
                    kwargs[param_name] = param.default.default
                # else: required file missing, will be caught by handler

        return kwargs

    @asynccontextmanager
    async def lifespan_context(self) -> AsyncGenerator[None, None]:
        """Context manager for application lifespan.

        Manages the complete application lifecycle including startup hooks,
        custom lifespan logic, and shutdown hooks.

        This follows the ASGI lifespan pattern, similar to FastAPI's
        @asynccontextmanager lifespan.

        Usage:
            async with app.lifespan_context():
                # Application is running
                # Do work here
                pass
            # Application is fully shut down

        Example:
            from contextlib import asynccontextmanager
            from ouroboros.api import App

            @asynccontextmanager
            async def lifespan(app: App):
                # Startup
                db = await connect_db()
                app.state.db = db
                yield
                # Shutdown
                await db.close()

            app = App(lifespan=lifespan)

            async with app.lifespan_context():
                # Application is ready, db is connected
                response = await app.state.db.query("SELECT 1")
            # Database is closed

        Yields:
            None: Control is yielded while the application is running
        """
        # Run startup hooks
        await self.startup()

        try:
            # Run custom lifespan if provided
            if self._lifespan:
                async with self._lifespan(self):
                    yield
            else:
                yield
        finally:
            # Run shutdown hooks
            await self.shutdown()

    def run(
        self,
        host: str = "127.0.0.1",
        port: int = 8000,
        use_rust_server: bool = True,
        reload: bool = False,
        workers: int = 1,
        log_level: str = "info",
        access_log: bool = True,
        **kwargs
    ) -> None:
        """Run the application.

        Args:
            host: Bind host (default: 127.0.0.1)
            port: Bind port (default: 8000)
            use_rust_server: Use high-performance Rust server (default: True)
                           If False, falls back to uvicorn with ASGI
            reload: Enable auto-reload on code changes (default: False)
                   Only works with uvicorn (use_rust_server=False)
            workers: Number of worker processes (default: 1)
                    Only works with uvicorn (use_rust_server=False)
            log_level: Logging level (default: "info")
            access_log: Enable access logging (default: True)
            **kwargs: Additional uvicorn config options (only with uvicorn)

        Example:
            >>> app = App(title="My API")
            >>> # Use Rust server (high performance, recommended)
            >>> app.run(host="0.0.0.0", port=8000)
            >>>
            >>> # Use uvicorn with ASGI (for development with reload)
            >>> app.run(host="0.0.0.0", port=8000, use_rust_server=False, reload=True)

        Note:
            - Rust server: Maximum performance, GIL-free request processing
            - Uvicorn/ASGI: Compatible with ASGI middleware, supports reload

            For production in K8s/containers, use Rust server:
            $ python -c "from app import app; app.run(host='0.0.0.0', port=8000)"

            Or use uvicorn CLI for ASGI compatibility:
            $ uvicorn app:app --host 0.0.0.0 --port 8000
        """
        if use_rust_server and self._rust_app is not None:
            # Use high-performance Rust server
            print(f"Starting data-bridge-api server on {host}:{port}")
            print("Using Rust HTTP server for maximum performance")
            print("Press Ctrl+C to shutdown")

            try:
                # This will block until Ctrl+C
                self._rust_app.serve(host, port)
            except KeyboardInterrupt:
                print("\nShutting down...")
        else:
            # Fall back to uvicorn with ASGI
            if use_rust_server:
                print("Warning: Rust server not available, falling back to uvicorn")

            try:
                import uvicorn
            except ImportError:
                raise ImportError(
                    "uvicorn is required to run the app. Install with: pip install uvicorn"
                )

            # Setup signal handlers for graceful shutdown
            setup_signal_handlers(self)

            # Build uvicorn config
            config = {
                "host": host,
                "port": port,
                "reload": reload,
                "workers": workers,
                "log_level": log_level,
                "access_log": access_log,
                **kwargs
            }

            print(f"Starting uvicorn server on {host}:{port}")
            print("Using ASGI interface (Python routing)")

            # Run with uvicorn
            uvicorn.run(self, **config)

    async def __call__(self, scope: dict, receive: Callable, send: Callable) -> None:
        """ASGI 3.0 interface for compatibility with uvicorn/hypercorn.

        This provides ASGI compatibility for running with standard ASGI servers:
        - uvicorn app:app
        - hypercorn app:app
        - gunicorn with uvicorn workers

        Note: This fallback uses Python routing and is slower than app.run().
        For best performance, use app.run(use_rust_server=True) which uses
        the Rust HTTP server with GIL-free request processing.
        """
        import json

        if scope["type"] != "http":
            # Only handle HTTP requests
            await send({
                "type": "http.response.start",
                "status": 404,
                "headers": [[b"content-type", b"text/plain"]],
            })
            await send({
                "type": "http.response.body",
                "body": b"Not found",
            })
            return

        # Extract request info
        method = scope["method"]
        path = scope["path"]
        headers = dict(scope.get("headers", []))

        # Find matching route (Python fallback routing)
        handler = None
        route_info = None
        for route in self._routes:
            if route.method.upper() == method and route.path == path:
                handler = route.handler
                route_info = route
                break

        if handler is None:
            # Route not found
            await send({
                "type": "http.response.start",
                "status": 404,
                "headers": [[b"content-type", b"application/json"]],
            })
            await send({
                "type": "http.response.body",
                "body": b'{"error": "Not found"}',
            })
            return

        # Call handler
        try:
            # Read body if present
            body = b""
            while True:
                message = await receive()
                if message["type"] == "http.request":
                    body += message.get("body", b"")
                    if not message.get("more_body", False):
                        break

            # Build request context
            from .dependencies import RequestContext
            context = RequestContext()
            context.scope = scope
            context.receive = receive
            context.send = send

            # Parse body if JSON
            body_data = None
            if body:
                content_type = headers.get(b"content-type", b"").decode()
                if "application/json" in content_type:
                    try:
                        body_data = json.loads(body.decode())
                    except json.JSONDecodeError:
                        pass

            # Resolve dependencies
            if not self._compiled:
                self.compile()

            resolved_deps = await self.resolve_dependencies(handler, context)

            # Build handler kwargs
            kwargs = {}

            # Extract parameters from handler signature
            sig = inspect.signature(handler)
            for param_name, param in sig.parameters.items():
                # Check if it's a dependency
                if param_name in resolved_deps:
                    kwargs[param_name] = resolved_deps[param_name]
                # Check if it's a path parameter
                elif param_name in scope.get("path_params", {}):
                    kwargs[param_name] = scope["path_params"][param_name]
                # Check if it's a query parameter
                elif param_name in scope.get("query_string", b"").decode():
                    # Simple query param extraction (not robust)
                    pass
                # Check for Body annotation
                elif body_data is not None:
                    kwargs[param_name] = body_data

            # Call handler
            result = await handler(**kwargs) if asyncio.iscoroutinefunction(handler) else handler(**kwargs)

            # Convert result to response
            if isinstance(result, Response):
                response_body = result.body
                status_code = result.status_code
                response_headers = [[k.encode(), v.encode()] for k, v in result.headers.items()]
            elif isinstance(result, dict):
                response_body = json.dumps(result).encode()
                status_code = route_info.status_code if route_info else 200
                response_headers = [[b"content-type", b"application/json"]]
            elif isinstance(result, str):
                response_body = result.encode()
                status_code = route_info.status_code if route_info else 200
                response_headers = [[b"content-type", b"text/plain"]]
            else:
                response_body = str(result).encode()
                status_code = route_info.status_code if route_info else 200
                response_headers = [[b"content-type", b"text/plain"]]

            # Send response
            await send({
                "type": "http.response.start",
                "status": status_code,
                "headers": response_headers,
            })
            await send({
                "type": "http.response.body",
                "body": response_body,
            })

        except HTTPException as e:
            # Handle HTTP exceptions
            await send({
                "type": "http.response.start",
                "status": e.status_code,
                "headers": [[b"content-type", b"application/json"]],
            })
            await send({
                "type": "http.response.body",
                "body": json.dumps({"error": e.detail}).encode(),
            })
        except Exception as e:
            # Handle unexpected errors
            import traceback
            traceback.print_exc()
            await send({
                "type": "http.response.start",
                "status": 500,
                "headers": [[b"content-type", b"application/json"]],
            })
            await send({
                "type": "http.response.body",
                "body": json.dumps({"error": "Internal server error", "detail": str(e)}).encode(),
            })


def setup_signal_handlers(app: "App") -> None:
    """Setup SIGTERM/SIGINT handlers for graceful shutdown in K8s."""
    import signal
    import asyncio

    def handle_signal(signum, frame):
        asyncio.create_task(app.shutdown())

    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)
