"""
Pydantic-style models for request/response validation.

This module re-exports from ouroboros.validation for backwards compatibility.
For new code, import directly from ouroboros.validation.

Example:
    from ouroboros.api.models import BaseModel, Field

    # Or preferably:
    from ouroboros.validation import BaseModel, Field

    class User(BaseModel):
        name: str = Field(min_length=1, max_length=100)
        age: int = Field(ge=0, le=150)
"""

# Re-export from ouroboros.validation
from ouroboros.validation import BaseModel, Field, ValidationError

# Also expose the type extraction functions for API server integration
from .type_extraction import extract_type_schema, schema_to_rust_type_descriptor

__all__ = [
    "BaseModel",
    "Field",
    "ValidationError",
    "extract_type_schema",
    "schema_to_rust_type_descriptor",
]
