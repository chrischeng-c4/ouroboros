"""
OpenAPI Schema Generation for data-bridge API

Generates OpenAPI 3.1 specifications from registered routes and type hints.
"""

from typing import Any, Callable, Dict, List, Optional, Type, get_type_hints, get_origin, get_args, Annotated
import inspect
import json

from .types import Path, Query, Body, Header
from .type_extraction import extract_type_schema


def generate_openapi(
    title: str,
    version: str,
    description: str = "",
    routes: List[Any] = None,
    servers: List[Dict[str, str]] = None,
    tags: List[Dict[str, str]] = None,
) -> Dict[str, Any]:
    """Generate OpenAPI 3.1 specification from routes.

    Args:
        title: API title
        version: API version
        description: API description
        routes: List of RouteInfo objects
        servers: List of server configurations
        tags: List of tag configurations

    Returns:
        OpenAPI 3.1 specification as dictionary
    """
    spec = {
        "openapi": "3.1.0",
        "info": {
            "title": title,
            "version": version,
        },
        "paths": {},
        "components": {
            "schemas": {},
        },
    }

    if description:
        spec["info"]["description"] = description

    if servers:
        spec["servers"] = servers

    if tags:
        spec["tags"] = tags

    if routes:
        for route in routes:
            add_route_to_spec(spec, route)

    return spec


def add_route_to_spec(spec: Dict[str, Any], route: Any) -> None:
    """Add a route to the OpenAPI spec."""
    path = convert_path_to_openapi(route.path)
    method = route.method.lower()

    if path not in spec["paths"]:
        spec["paths"][path] = {}

    operation = build_operation(route, spec["components"]["schemas"])
    spec["paths"][path][method] = operation


def convert_path_to_openapi(path: str) -> str:
    """Convert path from {param} to {param} format (already compatible)."""
    # FastAPI/data-bridge uses {param}, OpenAPI uses {param}
    # They're the same, but we could convert :param style if needed
    import re
    # Convert :param to {param} if present
    return re.sub(r':(\w+)', r'{\1}', path)


def build_operation(route: Any, schemas: Dict[str, Any]) -> Dict[str, Any]:
    """Build OpenAPI operation from route info."""
    operation: Dict[str, Any] = {
        "responses": {
            str(route.status_code): {
                "description": "Successful response",
            },
        },
    }

    if route.name:
        operation["operationId"] = route.name

    if route.summary:
        operation["summary"] = route.summary

    if route.description:
        operation["description"] = route.description

    if route.tags:
        operation["tags"] = route.tags

    if route.deprecated:
        operation["deprecated"] = True

    # Extract parameters and body from handler
    handler = route.handler
    parameters, request_body, response_schema = extract_operation_params(handler, schemas)

    if parameters:
        operation["parameters"] = parameters

    if request_body:
        operation["requestBody"] = request_body

    if response_schema:
        operation["responses"][str(route.status_code)]["content"] = {
            "application/json": {
                "schema": response_schema,
            },
        }

    return operation


def extract_operation_params(
    handler: Callable,
    schemas: Dict[str, Any],
) -> tuple:
    """Extract parameters, request body, and response schema from handler."""
    parameters: List[Dict[str, Any]] = []
    request_body: Optional[Dict[str, Any]] = None
    response_schema: Optional[Dict[str, Any]] = None

    try:
        hints = get_type_hints(handler, include_extras=True)
    except Exception:
        hints = {}

    sig = inspect.signature(handler)

    for param_name, param in sig.parameters.items():
        if param_name in ('self', 'cls', 'request', 'response'):
            continue

        hint = hints.get(param_name)
        if hint is None:
            continue

        origin = get_origin(hint)
        base_type = hint
        marker = None

        if origin is Annotated:
            args = get_args(hint)
            base_type = args[0]
            for arg in args[1:]:
                if isinstance(arg, (Path, Query, Body, Header)):
                    marker = arg
                    break

        type_schema = python_type_to_schema(base_type, schemas)

        if isinstance(marker, Path):
            parameters.append({
                "name": param_name,
                "in": "path",
                "required": True,
                "schema": type_schema,
                "description": getattr(marker, 'description', None) or None,
            })
        elif isinstance(marker, Query):
            param_info = {
                "name": param_name,
                "in": "query",
                "required": marker.default is ...,
                "schema": type_schema,
            }
            if marker.description:
                param_info["description"] = marker.description
            if marker.default is not ...:
                param_info["schema"]["default"] = marker.default
            parameters.append(param_info)
        elif isinstance(marker, Header):
            header_name = marker.alias or param_name.replace("_", "-").title()
            parameters.append({
                "name": header_name,
                "in": "header",
                "required": True,
                "schema": type_schema,
                "description": getattr(marker, 'description', None) or None,
            })
        elif isinstance(marker, Body):
            request_body = {
                "required": True,
                "content": {
                    marker.media_type: {
                        "schema": type_schema,
                    },
                },
            }
            if marker.description:
                request_body["description"] = marker.description
        else:
            # Default to query parameter
            param_info = {
                "name": param_name,
                "in": "query",
                "required": param.default is inspect.Parameter.empty,
                "schema": type_schema,
            }
            if param.default is not inspect.Parameter.empty:
                param_info["schema"]["default"] = param.default
            parameters.append(param_info)

    # Extract return type for response schema
    return_hint = hints.get('return')
    if return_hint and return_hint is not type(None):
        response_schema = python_type_to_schema(return_hint, schemas)

    # Clean up None values
    parameters = [
        {k: v for k, v in p.items() if v is not None}
        for p in parameters
    ]

    return parameters, request_body, response_schema


def python_type_to_schema(
    type_hint: Type,
    schemas: Dict[str, Any],
) -> Dict[str, Any]:
    """Convert Python type to OpenAPI schema."""
    schema = extract_type_schema(type_hint)
    return type_schema_to_openapi(schema, schemas)


def type_schema_to_openapi(
    schema: Dict[str, Any],
    schemas: Dict[str, Any],
) -> Dict[str, Any]:
    """Convert internal type schema to OpenAPI schema format."""
    type_name = schema.get("type", "any")

    if type_name == "string":
        result = {"type": "string"}
        if "min_length" in schema:
            result["minLength"] = schema["min_length"]
        if "max_length" in schema:
            result["maxLength"] = schema["max_length"]
        if "pattern" in schema:
            result["pattern"] = schema["pattern"]
        if schema.get("format") == "email":
            result["format"] = "email"
        if schema.get("format") == "uri":
            result["format"] = "uri"
        return result

    if type_name == "int":
        result = {"type": "integer"}
        if "minimum" in schema:
            result["minimum"] = schema["minimum"]
        if "maximum" in schema:
            result["maximum"] = schema["maximum"]
        return result

    if type_name == "float":
        result = {"type": "number"}
        if "minimum" in schema:
            result["minimum"] = schema["minimum"]
        if "maximum" in schema:
            result["maximum"] = schema["maximum"]
        return result

    if type_name == "bool":
        return {"type": "boolean"}

    if type_name == "list":
        items = schema.get("items", {"type": "any"})
        return {
            "type": "array",
            "items": type_schema_to_openapi(items, schemas),
        }

    if type_name == "object":
        result: Dict[str, Any] = {"type": "object"}

        if "fields" in schema:
            properties = {}
            required = []
            for field in schema["fields"]:
                properties[field["name"]] = type_schema_to_openapi(field["type"], schemas)
                if field.get("required", True):
                    required.append(field["name"])
            result["properties"] = properties
            if required:
                result["required"] = required

        # Check if it's a class with a name (for $ref)
        if "class_name" in schema:
            class_name = schema["class_name"]
            if class_name not in schemas:
                schemas[class_name] = result
            return {"$ref": f"#/components/schemas/{class_name}"}

        return result

    if type_name == "optional":
        inner = type_schema_to_openapi(schema.get("inner", {"type": "any"}), schemas)
        inner["nullable"] = True
        return inner

    if type_name == "union":
        variants = [type_schema_to_openapi(v, schemas) for v in schema.get("variants", [])]
        result = {"anyOf": variants}
        if schema.get("nullable"):
            result["nullable"] = True
        return result

    if type_name == "uuid":
        return {"type": "string", "format": "uuid"}

    if type_name == "datetime":
        return {"type": "string", "format": "date-time"}

    if type_name == "date":
        return {"type": "string", "format": "date"}

    if type_name == "time":
        return {"type": "string", "format": "time"}

    if type_name == "email":
        return {"type": "string", "format": "email"}

    if type_name == "url":
        return {"type": "string", "format": "uri"}

    if type_name == "enum":
        return {"enum": schema.get("values", [])}

    if type_name == "literal":
        return {"enum": schema.get("values", [])}

    # Default
    return {}


# Swagger UI HTML template
SWAGGER_UI_HTML = '''
<!DOCTYPE html>
<html>
<head>
    <title>{title} - Swagger UI</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        window.onload = function() {{
            SwaggerUIBundle({{
                url: "{openapi_url}",
                dom_id: '#swagger-ui',
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIBundle.SwaggerUIStandalonePreset
                ],
                layout: "StandaloneLayout"
            }});
        }};
    </script>
</body>
</html>
'''

# ReDoc HTML template
REDOC_HTML = '''
<!DOCTYPE html>
<html>
<head>
    <title>{title} - ReDoc</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
    <style>
        body {{ margin: 0; padding: 0; }}
    </style>
</head>
<body>
    <redoc spec-url='{openapi_url}'></redoc>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>
'''


def get_swagger_ui_html(title: str, openapi_url: str) -> str:
    """Get Swagger UI HTML page."""
    return SWAGGER_UI_HTML.format(title=title, openapi_url=openapi_url)


def get_redoc_html(title: str, openapi_url: str) -> str:
    """Get ReDoc HTML page."""
    return REDOC_HTML.format(title=title, openapi_url=openapi_url)
