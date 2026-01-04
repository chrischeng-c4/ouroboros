"""
Main App class for the API framework.
"""

from typing import (
    Any, Callable, Dict, List, Optional, Type, TypeVar, Union,
    get_type_hints, get_origin, get_args, Annotated
)
import inspect
import functools
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

# Import Rust bindings
try:
    from data_bridge._engine import api as _api
except ImportError:
    _api = None

T = TypeVar('T')

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
    ):
        self.title = title
        self.version = version
        self.description = description
        self.docs_url = docs_url
        self.redoc_url = redoc_url
        self.openapi_url = openapi_url

        self._routes: List[RouteInfo] = []
        self._handlers: Dict[str, Callable] = {}
        self._dependency_container = DependencyContainer()
        self._compiled = False
        self._global_deps: Dict[int, str] = {}  # Track factory id -> registered name
        self._docs_setup = False
        self._http_provider = HttpClientProvider()

        # Initialize Rust app if available
        if _api is not None:
            self._rust_app = _api.ApiApp(title=title, version=version)
        else:
            self._rust_app = None

    def route(
        self,
        path: str,
        *,
        methods: List[str] = None,
        name: str = None,
        summary: str = None,
        description: str = None,
        tags: List[str] = None,
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
        name: str = None,
        summary: str = None,
        description: str = None,
        tags: List[str] = None,
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
        name: str = None,
        summary: str = None,
        description: str = None,
        tags: List[str] = None,
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
        name: str = None,
        summary: str = None,
        description: str = None,
        tags: List[str] = None,
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
        name: str = None,
        summary: str = None,
        description: str = None,
        tags: List[str] = None,
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
        name: str = None,
        summary: str = None,
        description: str = None,
        tags: List[str] = None,
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

            self._rust_app.register_route(
                method=method,
                path=path,
                handler=handler,
                validator_dict=meta.get("validator"),
                metadata_dict={
                    "operation_id": name,
                    "summary": summary,
                    "description": description,
                    "tags": tags,
                    "deprecated": deprecated,
                    "status_code": status_code,
                },
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
