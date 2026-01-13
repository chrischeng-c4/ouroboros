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
import threading
import typing as _typing
from typing import TYPE_CHECKING

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

__all__ = ["PyLoop", "install", "EventLoopPolicy", "is_installed", "App"]

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

    def __init__(self, title: str = "DataBridge API", version: str = "0.1.0"):
        """Initialize the App.

        Args:
            title: API title for OpenAPI documentation
            version: API version for OpenAPI documentation
        """
        if _RustApp is None:
            raise RuntimeError("PyLoop Rust extension not available")
        self._app = _RustApp(title=title, version=version)

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
            self._app.register_route("GET", path, func)
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
            self._app.register_route("POST", path, func)
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
            self._app.register_route("PUT", path, func)
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
            self._app.register_route("PATCH", path, func)
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
            self._app.register_route("DELETE", path, func)
            return func
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
