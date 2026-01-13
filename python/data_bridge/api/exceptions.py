"""
HTTP exceptions for API handlers.
"""

from typing import Any, Dict, Optional

class HTTPException(Exception):
    """HTTP exception with status code and detail."""

    def __init__(
        self,
        status_code: int,
        detail: Any = None,
        headers: Optional[Dict[str, str]] = None,
    ):
        self.status_code = status_code
        self.detail = detail
        self.headers = headers
        super().__init__(detail)

    def __repr__(self) -> str:
        return f"HTTPException(status_code={self.status_code}, detail={self.detail!r})"
