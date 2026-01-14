"""
Type markers for request parameters.

These types are used with typing.Annotated to specify
where parameters come from and their validation constraints.
"""

from typing import Any, Optional, TypeVar, Generic
from dataclasses import dataclass, field
from ouroboros.api.dependencies import Depends

T = TypeVar('T')

@dataclass
class Path:
    """Mark a parameter as path parameter.

    Example:
        @app.get("/users/{user_id}")
        async def get_user(
            user_id: Annotated[str, Path(description="User ID")]
        ) -> User:
            ...
    """
    description: str = ""
    example: Any = None
    deprecated: bool = False

@dataclass
class Query:
    """Mark a parameter as query parameter.

    Example:
        @app.get("/users")
        async def list_users(
            skip: Annotated[int, Query(default=0, ge=0)] = 0,
            limit: Annotated[int, Query(default=20, le=100)] = 20,
        ) -> List[User]:
            ...
    """
    default: Any = ...  # ... means required
    description: str = ""
    example: Any = None
    deprecated: bool = False
    # Constraints
    ge: Optional[float] = None  # greater than or equal
    gt: Optional[float] = None  # greater than
    le: Optional[float] = None  # less than or equal
    lt: Optional[float] = None  # less than
    min_length: Optional[int] = None
    max_length: Optional[int] = None
    pattern: Optional[str] = None

@dataclass
class Body:
    """Mark a parameter as request body.

    Example:
        @app.post("/users")
        async def create_user(
            user: Annotated[UserCreate, Body()]
        ) -> User:
            ...
    """
    media_type: str = "application/json"
    description: str = ""
    example: Any = None
    embed: bool = False  # If True, expect body key

@dataclass
class Header:
    """Mark a parameter as header.

    Example:
        @app.get("/me")
        async def get_me(
            authorization: Annotated[str, Header()]
        ) -> User:
            ...
    """
    alias: Optional[str] = None  # Header name (if different from param name)
    description: str = ""
    deprecated: bool = False
