"""Request context for data-bridge-api."""
from typing import Optional, Any, Dict
from dataclasses import dataclass, field
import uuid


@dataclass
class RequestContext:
    """Context object available during request handling.

    Provides access to request metadata, headers, parameters, and
    the configured HTTP client for making external requests.

    Example:
        @app.get("/proxy")
        async def proxy_handler(ctx: RequestContext):
            # Access HTTP client
            response = await ctx.http.get("/external-api")
            return response.json()

    Attributes:
        request_id: Unique identifier for this request
        client_ip: Client's IP address
        method: HTTP method (GET, POST, etc.)
        path: Request path
        headers: Request headers (dict)
        path_params: Path parameters extracted from route
        query_params: Query string parameters
    """

    # Request metadata
    request_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    client_ip: Optional[str] = None
    method: str = ""
    path: str = ""

    # Headers and params
    headers: Dict[str, str] = field(default_factory=dict)
    path_params: Dict[str, Any] = field(default_factory=dict)
    query_params: Dict[str, Any] = field(default_factory=dict)

    # HTTP client (injected)
    _http_client: Optional[Any] = field(default=None, repr=False)

    @property
    def http(self):
        """Get the HTTP client for making external requests.

        Returns:
            Configured HttpClient instance

        Raises:
            RuntimeError: If HTTP client not configured

        Example:
            response = await ctx.http.get("/api/data")
            data = response.json()
        """
        if self._http_client is None:
            raise RuntimeError(
                "HTTP client not configured. "
                "Call app.configure_http_client() first."
            )
        return self._http_client

    def get_header(self, name: str, default: Optional[str] = None) -> Optional[str]:
        """Get a header value (case-insensitive lookup).

        Args:
            name: Header name
            default: Default value if not found

        Returns:
            Header value or default

        Example:
            auth = ctx.get_header("Authorization")
            content_type = ctx.get_header("content-type", "application/json")
        """
        name_lower = name.lower()
        for key, value in self.headers.items():
            if key.lower() == name_lower:
                return value
        return default
