"""HTTP client integration for data-bridge-api."""
from typing import Optional, Any
from ..http import HttpClient


def create_http_client(
    base_url: Optional[str] = None,
    timeout: float = 30.0,
    connect_timeout: float = 10.0,
    **kwargs
) -> HttpClient:
    """Create an HTTP client instance with configuration.

    Args:
        base_url: Base URL for all requests
        timeout: Total request timeout in seconds
        connect_timeout: Connection timeout in seconds
        **kwargs: Additional HttpClient configuration

    Returns:
        Configured HttpClient instance
    """
    return HttpClient(
        base_url=base_url,
        timeout=timeout,
        connect_timeout=connect_timeout,
        **kwargs
    )


class HttpClientProvider:
    """Provides HTTP client as a singleton dependency.

    This provider manages a single HttpClient instance per application,
    configured via App.configure_http_client().

    Example:
        app = App()
        app.configure_http_client(base_url="https://api.example.com")

        # Access via dependency injection
        async def handler(http: HttpClient = Depends()):
            response = await http.get("/data")
            return response.json()
    """

    def __init__(self):
        self._client: Optional[HttpClient] = None
        self._config: dict = {}

    def configure(
        self,
        base_url: Optional[str] = None,
        timeout: float = 30.0,
        connect_timeout: float = 10.0,
        **kwargs
    ):
        """Configure the HTTP client.

        Args:
            base_url: Base URL for all requests
            timeout: Total request timeout in seconds
            connect_timeout: Connection timeout in seconds
            **kwargs: Additional HttpClient configuration (pool_max_idle_per_host, etc.)
        """
        self._config = {
            "base_url": base_url,
            "timeout": timeout,
            "connect_timeout": connect_timeout,
            **kwargs
        }
        # Reset client so it gets recreated with new config
        self._client = None

    def get_client(self) -> HttpClient:
        """Get or create the HTTP client singleton.

        Returns:
            Configured HttpClient instance
        """
        if self._client is None:
            self._client = create_http_client(**self._config)
        return self._client

    def __call__(self) -> HttpClient:
        """Callable for dependency injection.

        Returns:
            Configured HttpClient instance
        """
        return self.get_client()

    async def close(self) -> None:
        """Close the HTTP client and release resources.

        This should be called during application shutdown to properly
        clean up HTTP connections.
        """
        if self._client is not None:
            # HttpClient cleanup - no explicit close needed as Rust handles it
            # Just clear the reference
            self._client = None
