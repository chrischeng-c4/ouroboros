"""
ouroboros.validation - Standalone validation module

A high-performance validation library that replaces Pydantic without any external
dependencies. Uses Rust for validation to achieve maximum performance.

Example:
    from ouroboros.validation import BaseModel, Field
    from typing import Annotated

    class User(BaseModel):
        name: Annotated[str, Field(min_length=1, max_length=100)]
        age: Annotated[int, Field(ge=0, le=150)]
        email: str

    user = User(name="John", age=30, email="john@example.com")
    data = user.model_dump()
"""

from .models import BaseModel, Field, ValidationError

__all__ = [
    "BaseModel",
    "Field",
    "ValidationError",
]
