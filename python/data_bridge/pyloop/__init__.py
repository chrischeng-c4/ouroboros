"""
data_bridge.pyloop - Rust-native asyncio event loop

Provides a high-performance drop-in replacement for Python's asyncio event loop,
backed by Tokio runtime with seamless Rust integration.

Example:
    >>> import data_bridge.pyloop
    >>> data_bridge.pyloop.install()
    >>> import asyncio
    >>> # Now using Tokio-backed event loop!

Architecture:
    Python asyncio protocol → PyLoop (PyO3) → Tokio Runtime (Rust)

Benefits:
    - 2-10x faster event loop operations
    - Better integration with Rust async code
    - Reduced GIL contention
    - Native support for spawning Rust futures
"""

from __future__ import annotations

import asyncio
import logging
import threading
import typing as _typing
from typing import TYPE_CHECKING, Optional, Any, Dict

# Configure logger
logger = logging.getLogger("data_bridge.pyloop")

if TYPE_CHECKING:
    # Type stubs for the extension module
    from asyncio import AbstractEventLoop

    class PyLoop(AbstractEventLoop):
        """Type stub for PyLoop class from Rust extension."""
        ...
else:
    # Import the Rust extension module at runtime
    try:
        import data_bridge.data_bridge as _db_native  # type: ignore[import-not-found]
        _pyloop = _db_native._pyloop  # type: ignore[attr-defined]
        _api = _db_native.api  # type: ignore[attr-defined]
        PyLoop = _pyloop.PyLoop
        _RustApp = _api.ApiApp  # Use ApiApp from the api module
    except (ImportError, AttributeError) as e:
        raise ImportError(
            "Failed to import PyLoop from data_bridge native module. "
            "Please run 'maturin develop' to build the extension."
        ) from e
        _RustApp = None

__all__ = [
    "PyLoop",
    "install",
    "EventLoopPolicy",
    "is_installed",
    "App",
    "HTTPException",
    "ValidationError",
    "NotFoundError",
    "ConflictError"
]

__version__ = "0.1.0"

_AbstractEventLoop = asyncio.AbstractEventLoop


class EventLoopPolicy(
    # This is to avoid a mypy error about AbstractEventLoopPolicy
    getattr(asyncio, 'AbstractEventLoopPolicy')  # type: ignore[misc]
):
    """Custom event loop policy that returns PyLoop instances.

    This class implements the asyncio.AbstractEventLoopPolicy interface
    to provide PyLoop instances as the default event loop.

    Example:
        >>> import asyncio
        >>> from data_bridge.pyloop import EventLoopPolicy
        >>> asyncio.set_event_loop_policy(EventLoopPolicy())
        >>> loop = asyncio.get_event_loop()
        >>> # loop is now a PyLoop instance
    """

    def _loop_factory(self) -> _AbstractEventLoop:
        """Factory method to create a new event loop.

        Returns:
            AbstractEventLoop: A new PyLoop instance.
        """
        return PyLoop()

    if _typing.TYPE_CHECKING:
        # EventLoopPolicy doesn't implement these, but since they are
        # marked as abstract in typeshed, we have to put them in so mypy
        # thinks the base methods are overridden. This is the same approach
        # taken for the Windows event loop policy classes in typeshed.
        def get_child_watcher(self) -> _typing.NoReturn:
            ...

        def set_child_watcher(
            self, watcher: _typing.Any
        ) -> _typing.NoReturn:
            ...

    class _Local(threading.local):
        _loop: _typing.Optional[_AbstractEventLoop] = None

    def __init__(self) -> None:
        self._local = self._Local()

    def get_event_loop(self) -> _AbstractEventLoop:
        """Get the event loop for the current context.

        Returns an instance of EventLoop or raises an exception.
        """
        if self._local._loop is None:
            # Auto-create a loop if one doesn't exist (matches asyncio behavior)
            self.set_event_loop(self.new_event_loop())

        assert self._local._loop is not None  # Help type checker
        return self._local._loop

    def set_event_loop(
        self, loop: _typing.Optional[_AbstractEventLoop]
    ) -> None:
        """Set the event loop."""
        # Accept PyLoop (Rust type) or Python's AbstractEventLoop
        if loop is not None and not isinstance(loop, (PyLoop, _AbstractEventLoop)):
            raise TypeError(
                f"loop must be an instance of AbstractEventLoop or None, "
                f"not '{type(loop).__name__}'"
            )
        self._local._loop = loop

    def new_event_loop(self) -> _AbstractEventLoop:
        """Create a new event loop.

        You must call set_event_loop() to make this the current event loop.
        """
        return self._loop_factory()


def install() -> None:
    """Install PyLoop as the default asyncio event loop.

    This replaces the standard asyncio event loop policy with our
    Tokio-backed implementation. All subsequent asyncio operations
    will use PyLoop automatically.

    This should be called early in your application, typically in
    the main module before any asyncio code runs.

    Example:
        >>> import data_bridge.pyloop
        >>> data_bridge.pyloop.install()
        >>> import asyncio
        >>> loop = asyncio.get_event_loop()
        >>> # loop is now a PyLoop instance backed by Tokio

    Note:
        This is a global operation that affects all asyncio code in
        the current process. It should only be called once.
    """
    asyncio.set_event_loop_policy(EventLoopPolicy())  # type: ignore[arg-type]


def is_installed() -> bool:
    """Check if PyLoop is currently installed as the default event loop.

    Returns:
        bool: True if PyLoop is installed, False otherwise.

    Example:
        >>> import data_bridge.pyloop
        >>> data_bridge.pyloop.is_installed()
        False
        >>> data_bridge.pyloop.install()
        >>> data_bridge.pyloop.is_installed()
        True
    """
    policy = asyncio.get_event_loop_policy()
    return isinstance(policy, EventLoopPolicy)


class HTTPException(Exception):
    """
    HTTP exception with status code and detail.

    Raise this in handlers to return specific HTTP status codes.

    Example:
        raise HTTPException(404, "Product not found")
        raise HTTPException(400, "Invalid request", {"field": "price"})
    """
    def __init__(
        self,
        status_code: int,
        detail: Optional[str] = None,
        headers: Optional[Dict[str, str]] = None,
        extra: Optional[Dict[str, Any]] = None
    ):
        self.status_code = status_code
        self.detail = detail or self._default_detail(status_code)
        self.headers = headers or {}
        self.extra = extra or {}
        super().__init__(self.detail)

    @staticmethod
    def _default_detail(status_code: int) -> str:
        """Get default detail message for status code."""
        messages = {
            400: "Bad Request",
            401: "Unauthorized",
            403: "Forbidden",
            404: "Not Found",
            405: "Method Not Allowed",
            409: "Conflict",
            422: "Unprocessable Entity",
            500: "Internal Server Error",
            503: "Service Unavailable"
        }
        return messages.get(status_code, "Error")

    def to_response(self) -> Dict[str, Any]:
        """Convert exception to response dict."""
        body = {
            "error": self.detail,
            "status_code": self.status_code
        }
        if self.extra:
            body.update(self.extra)

        response = {
            "status": self.status_code,
            "body": body
        }

        if self.headers:
            response["headers"] = self.headers

        return response


class ValidationError(HTTPException):
    """Validation error (422 Unprocessable Entity)."""
    def __init__(self, detail: str, errors: Optional[Dict] = None):
        super().__init__(422, detail, extra={"errors": errors} if errors else None)


class NotFoundError(HTTPException):
    """Not found error (404)."""
    def __init__(self, detail: str = "Resource not found"):
        super().__init__(404, detail)


class ConflictError(HTTPException):
    """Conflict error (409) - e.g., duplicate key."""
    def __init__(self, detail: str = "Resource already exists"):
        super().__init__(409, detail)


class App:
    """
    Simple FastAPI-style app builder for PyLoop.

    Provides a decorator-based API for registering Python handlers that execute
    on the Rust-backed HTTP server with async/sync support.

    Example:
        >>> from data_bridge.pyloop import App
        >>>
        >>> app = App(title="My API", version="1.0.0")
        >>>
        >>> @app.get("/users/{user_id}")
        >>> async def get_user(path_params):
        ...     return {"user_id": path_params["user_id"]}
        >>>
        >>> @app.post("/users")
        >>> async def create_user(body):
        ...     return {"id": "new_id", **body}
        >>>
        >>> # Run the server
        >>> app.serve(host="127.0.0.1", port=8000)
    """

    def __init__(self, title: str = "DataBridge API", version: str = "0.1.0", debug: bool = False):
        """Initialize the App.

        Args:
            title: API title for OpenAPI documentation
            version: API version for OpenAPI documentation
            debug: Enable debug mode (exposes stack traces)
        """
        if _RustApp is None:
            raise RuntimeError("PyLoop Rust extension not available")
        self._app = _RustApp(title=title, version=version)
        self.debug = debug

    def _handle_error(self, error: Exception, request: Dict = None) -> Dict[str, Any]:
        """
        Convert Python exception to HTTP response.

        Args:
            error: The exception that was raised
            request: The request dict (for logging context)

        Returns:
            Response dict with appropriate status code and error message
        """
        # HTTPException - already formatted
        if isinstance(error, HTTPException):
            if not isinstance(error, (ValidationError, NotFoundError)):
                # Log non-trivial errors
                logger.warning(
                    f"HTTP {error.status_code}: {error.detail}",
                    extra={"path": request.get("path") if request else None}
                )
            return error.to_response()

        # MongoDB errors (from data-bridge)
        error_str = str(error)

        # Duplicate key error (MongoDB)
        if "duplicate key" in error_str.lower() or "E11000" in error_str:
            logger.warning(f"Duplicate key error: {error_str}")
            return {
                "status": 409,
                "body": {
                    "error": "Resource already exists",
                    "type": "ConflictError"
                }
            }

        # Validation errors (MongoDB/Pydantic)
        if "validation" in error_str.lower() or "ValidationError" in type(error).__name__:
            logger.warning(f"Validation error: {error_str}")
            return {
                "status": 422,
                "body": {
                    "error": "Validation failed",
                    "detail": error_str if self.debug else "Invalid request data",
                    "type": "ValidationError"
                }
            }

        # ObjectId errors (invalid ID format)
        if "ObjectId" in error_str or "invalid objectid" in error_str.lower():
            return {
                "status": 400,
                "body": {
                    "error": "Invalid ID format",
                    "type": "BadRequest"
                }
            }

        # Generic error - don't expose internals in production
        logger.error(f"Unhandled error: {type(error).__name__}: {error_str}", exc_info=True)

        if self.debug:
            # Debug mode: include full error details
            import traceback
            return {
                "status": 500,
                "body": {
                    "error": "Internal Server Error",
                    "type": type(error).__name__,
                    "detail": error_str,
                    "traceback": traceback.format_exc()
                }
            }
        else:
            # Production: generic error message
            return {
                "status": 500,
                "body": {
                    "error": "Internal Server Error",
                    "type": "InternalServerError"
                }
            }

    def _wrap_handler_with_error_handling(self, handler):
        """
        Wrap a handler function with error handling.

        Args:
            handler: The async handler function

        Returns:
            Wrapped handler that catches and converts exceptions
        """
        async def wrapped_handler(request):
            try:
                return await handler(request)
            except Exception as e:
                return self._handle_error(e, request)

        return wrapped_handler

    def get(self, path: str):
        """Register a GET route handler.

        Args:
            path: URL path pattern (e.g., "/users/{user_id}")

        Returns:
            Decorator function for handler registration

        Example:
            >>> @app.get("/status")
            >>> async def get_status(path_params, query_params, headers, body):
            ...     return {"status": "ok"}
        """
        def decorator(func):
            # Wrap with error handling
            wrapped = self._wrap_handler_with_error_handling(func)
            self._app.register_route("GET", path, wrapped)
            return func
        return decorator

    def post(self, path: str):
        """Register a POST route handler.

        Args:
            path: URL path pattern (e.g., "/users")

        Returns:
            Decorator function for handler registration

        Example:
            >>> @app.post("/users")
            >>> async def create_user(path_params, query_params, headers, body):
            ...     return {"id": "new_id", **body}
        """
        def decorator(func):
            # Wrap with error handling
            wrapped = self._wrap_handler_with_error_handling(func)
            self._app.register_route("POST", path, wrapped)
            return func
        return decorator

    def put(self, path: str):
        """Register a PUT route handler.

        Args:
            path: URL path pattern (e.g., "/users/{user_id}")

        Returns:
            Decorator function for handler registration

        Example:
            >>> @app.put("/users/{user_id}")
            >>> async def update_user(path_params, query_params, headers, body):
            ...     return {"id": path_params["user_id"], **body}
        """
        def decorator(func):
            # Wrap with error handling
            wrapped = self._wrap_handler_with_error_handling(func)
            self._app.register_route("PUT", path, wrapped)
            return func
        return decorator

    def patch(self, path: str):
        """Register a PATCH route handler.

        Args:
            path: URL path pattern (e.g., "/users/{user_id}")

        Returns:
            Decorator function for handler registration

        Example:
            >>> @app.patch("/users/{user_id}")
            >>> async def partial_update_user(path_params, query_params, headers, body):
            ...     return {"id": path_params["user_id"], "updated": True}
        """
        def decorator(func):
            # Wrap with error handling
            wrapped = self._wrap_handler_with_error_handling(func)
            self._app.register_route("PATCH", path, wrapped)
            return func
        return decorator

    def delete(self, path: str):
        """Register a DELETE route handler.

        Args:
            path: URL path pattern (e.g., "/users/{user_id}")

        Returns:
            Decorator function for handler registration

        Example:
            >>> @app.delete("/users/{user_id}")
            >>> async def delete_user(path_params, query_params, headers, body):
            ...     return {"deleted": True, "user_id": path_params["user_id"]}
        """
        def decorator(func):
            # Wrap with error handling
            wrapped = self._wrap_handler_with_error_handling(func)
            self._app.register_route("DELETE", path, wrapped)
            return func
        return decorator

    def crud_routes(
        self,
        document_cls,
        prefix: Optional[str] = None,
        *,
        operations: Optional[str] = None,
        tags: Optional[list] = None,
        create: bool = True,
        read: bool = True,
        update: bool = True,
        delete: bool = True,
        list: bool = True,
    ):
        """
        Auto-generate CRUD endpoints for a Document model.

        Args:
            document_cls: Document class to generate CRUD for
            prefix: URL prefix (default: /collection_name)
                   Can be passed as positional argument: app.crud_routes(Product, "/products")
            operations: String specifying operations (e.g., "CRUDL", "CR", "RUD")
                       C=Create, R=Read, U=Update, D=Delete, L=List
                       If provided, overrides individual flags
            tags: OpenAPI tags (default: [collection_name])
            create: Generate POST endpoint (default: True)
            read: Generate GET /{id} endpoint (default: True)
            update: Generate PUT /{id} endpoint (default: True)
            delete: Generate DELETE /{id} endpoint (default: True)
            list: Generate GET / endpoint with pagination (default: True)

        Example:
            # All operations with default prefix
            app.crud_routes(Product)

            # Custom prefix (positional argument - recommended)
            app.crud_routes(Product, "/products")

            # With operation string
            app.crud_routes(Product, "/products", operations="RL")

            # Only read operations
            app.crud_routes(Product, operations="RL")

            # Explicit flags
            app.crud_routes(Product, "/products", create=True, read=True, list=False)
        """
        # Parse operations string if provided
        if operations is not None:
            operations = operations.upper()
            create = "C" in operations
            read = "R" in operations
            update = "U" in operations
            delete = "D" in operations
            list = "L" in operations

        # Get collection name from Document class
        if hasattr(document_cls, '__collection_name__'):
            collection_name = document_cls.__collection_name__()
        elif hasattr(document_cls, '_collection_name'):
            collection_name = document_cls._collection_name
        else:
            collection_name = document_cls.__name__.lower()

        # Default prefix to /collection_name
        if prefix is None:
            prefix = f"/{collection_name}"

        # Default tags
        if tags is None:
            tags = [collection_name]

        # Generate LIST endpoint: GET /resource?skip=0&limit=10
        async def list_handler(request):
            """List documents with pagination."""
            query_params = request.get("query_params", {})

            # Extract pagination params
            skip = int(query_params.get("skip", 0))
            limit = int(query_params.get("limit", 10))
            limit = min(limit, 100)  # Cap at 100

            # Execute query
            documents = await document_cls.find().skip(skip).limit(limit).to_list()

            # Serialize to dict
            items = [doc.to_dict() for doc in documents]

            return {
                "status": 200,
                "body": {
                    "items": items,
                    "skip": skip,
                    "limit": limit,
                    "total": len(items)
                }
            }

        # Generate GET_BY_ID endpoint: GET /resource/{id}
        async def get_handler(request):
            """Get document by ID."""
            doc_id = request["path_params"]["id"]

            try:
                document = await document_cls.get(doc_id)
                if document is None:
                    raise NotFoundError(f"{collection_name.capitalize()} not found")

                return {"status": 200, "body": document.to_dict()}
            except HTTPException:
                raise  # Re-raise HTTPException as-is
            except Exception as e:
                # Let the error handler deal with it
                raise

        # Generate CREATE endpoint: POST /resource
        async def create_handler(request):
            """Create new document."""
            body = request.get("body", {})

            if not body:
                raise ValidationError("Request body required")

            try:
                document = document_cls(**body)
                await document.save()
                return {"status": 201, "body": document.to_dict()}
            except HTTPException:
                raise
            except Exception as e:
                raise  # Will be caught by error handler

        # Generate UPDATE endpoint: PUT /resource/{id}
        async def update_handler(request):
            """Update document by ID."""
            doc_id = request["path_params"]["id"]
            body = request.get("body", {})

            if not body:
                raise ValidationError("Request body required")

            try:
                document = await document_cls.get(doc_id)
                if document is None:
                    raise NotFoundError(f"{collection_name.capitalize()} not found")

                for key, value in body.items():
                    if not key.startswith('_'):
                        setattr(document, key, value)

                await document.save()
                return {"status": 200, "body": document.to_dict()}
            except HTTPException:
                raise
            except Exception as e:
                raise

        # Generate DELETE endpoint: DELETE /resource/{id}
        async def delete_handler(request):
            """Delete document by ID."""
            doc_id = request["path_params"]["id"]

            try:
                document = await document_cls.get(doc_id)
                if document is None:
                    raise NotFoundError(f"{collection_name.capitalize()} not found")

                await document.delete()
                return {"status": 204, "body": None}
            except HTTPException:
                raise
            except Exception as e:
                raise

        # Register endpoints based on flags
        if list:
            self.get(f"{prefix}")(list_handler)

        if read:
            self.get(f"{prefix}/{{id}}")(get_handler)

        if create:
            self.post(f"{prefix}")(create_handler)

        if update:
            self.put(f"{prefix}/{{id}}")(update_handler)

        if delete:
            self.delete(f"{prefix}/{{id}}")(delete_handler)

    def crud(self, document_cls, prefix: Optional[str] = None, tags: Optional[list] = None):
        """
        Legacy decorator-style CRUD generation (deprecated).

        Use crud_routes() instead for direct method call:
            app.crud_routes(Product)

        This method is kept for backward compatibility.

        Args:
            document_cls: Document class to generate CRUD for
            prefix: URL prefix (default: /collection_name)
            tags: OpenAPI tags (default: [collection_name])

        Returns:
            Decorator function (for @app.crud(Model) syntax)

        Example:
            from data_bridge.mongodb import Document
            from data_bridge.pyloop import App

            class Product(Document):
                name: str
                price: float

                class Settings:
                    name = "products"

            app = App()

            @app.crud(Product)
            class ProductCRUD:
                pass  # Endpoints auto-generated

            # This generates:
            # GET /products?skip=0&limit=10 - List products
            # GET /products/{id} - Get product
            # POST /products - Create product
            # PUT /products/{id} - Update product
            # DELETE /products/{id} - Delete product
        """
        # Call the new crud_routes method
        self.crud_routes(document_cls, prefix=prefix, tags=tags)

        # Return decorator for @app.crud(Model) syntax
        def decorator(cls):
            return cls
        return decorator

    def serve(self, host: str = "127.0.0.1", port: int = 8000):
        """
        Start the HTTP server (blocking).

        This method will block until the server receives a shutdown signal (Ctrl+C).
        The server runs with the GIL released for maximum performance.

        Args:
            host: Bind address (default: 127.0.0.1)
            port: Bind port (default: 8000)

        Example:
            >>> app.serve(host="0.0.0.0", port=3000)
        """
        print(f"Starting server on http://{host}:{port}")
        print("Press Ctrl+C to stop")
        self._app.serve(host, port)
