"""
Endpoint configuration for route definitions.

Provides the Endpoint class for defining API endpoints as Single Source of Truth (SSOT).
Route handlers and tests reference these endpoint definitions.

Example:
    from ouroboros.api import Endpoint, HTTPMethod, HTTPStatus
    from .schemas import TodoCreate, TodoResponse, TodoListResponse

    class TodoEndpoints:
        PREFIX = "/todo"

        LIST = Endpoint(
            path="/",
            method=HTTPMethod.GET,
            handler="list_todos",
            summary="List all todos",
            response_model=TodoListResponse,
        )
        CREATE = Endpoint(
            path="/",
            method=HTTPMethod.POST,
            handler="create_todo",
            summary="Create a todo",
            status_code=HTTPStatus.CREATED,
            request_model=TodoCreate,
            response_model=TodoResponse,
        )
"""

from http import HTTPMethod, HTTPStatus
from typing import Type

from ouroboros.validation import BaseModel


class Endpoint(BaseModel):
    """Configuration for a single API endpoint.

    Defines the path, method, handler, and schema information for an endpoint.
    Used as SSOT - route handlers and tests reference these definitions.

    Attributes:
        path: URL path pattern (e.g., "/", "/{id}")
        method: HTTP method (GET, POST, PUT, DELETE, etc.)
        handler: Handler function name
        summary: Human-readable description for OpenAPI docs
        status_code: HTTP status code for successful response
        request_model: Schema class for request body validation
        response_model: Schema class for response serialization
    """

    path: str
    method: HTTPMethod
    handler: str
    summary: str = ""
    status_code: HTTPStatus = HTTPStatus.OK
    request_model: Type | None = None
    response_model: Type | None = None
