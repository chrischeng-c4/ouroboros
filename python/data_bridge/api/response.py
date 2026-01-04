"""
Response classes for API handlers.
"""

from typing import Any, Dict, Optional
import json

class Response:
    """Base response class."""

    def __init__(
        self,
        content: Any = None,
        status_code: int = 200,
        headers: Dict[str, str] = None,
        media_type: str = None,
    ):
        self.content = content
        self.status_code = status_code
        self.headers = headers or {}
        self.media_type = media_type

    def set_cookie(
        self,
        key: str,
        value: str,
        max_age: int = None,
        expires: int = None,
        path: str = "/",
        domain: str = None,
        secure: bool = False,
        httponly: bool = True,
        samesite: str = "lax",
    ) -> "Response":
        """Set a cookie."""
        cookie = f"{key}={value}; Path={path}"
        if max_age is not None:
            cookie += f"; Max-Age={max_age}"
        if domain:
            cookie += f"; Domain={domain}"
        if secure:
            cookie += "; Secure"
        if httponly:
            cookie += "; HttpOnly"
        if samesite:
            cookie += f"; SameSite={samesite}"

        self.headers["Set-Cookie"] = cookie
        return self

    def delete_cookie(
        self,
        key: str,
        path: str = "/",
        domain: str = None,
    ) -> "Response":
        """Delete a cookie."""
        return self.set_cookie(key, "", max_age=0, path=path, domain=domain)


class JSONResponse(Response):
    """JSON response."""

    def __init__(
        self,
        content: Any,
        status_code: int = 200,
        headers: Dict[str, str] = None,
    ):
        super().__init__(
            content=content,
            status_code=status_code,
            headers=headers,
            media_type="application/json",
        )

    def body(self) -> bytes:
        """Get response body as bytes."""
        return json.dumps(self.content).encode("utf-8")


class HTMLResponse(Response):
    """HTML response."""

    def __init__(
        self,
        content: str,
        status_code: int = 200,
        headers: Dict[str, str] = None,
    ):
        super().__init__(
            content=content,
            status_code=status_code,
            headers=headers,
            media_type="text/html; charset=utf-8",
        )


class PlainTextResponse(Response):
    """Plain text response."""

    def __init__(
        self,
        content: str,
        status_code: int = 200,
        headers: Dict[str, str] = None,
    ):
        super().__init__(
            content=content,
            status_code=status_code,
            headers=headers,
            media_type="text/plain; charset=utf-8",
        )
