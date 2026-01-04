"""
Enhanced type extraction for complex Python types.

Supports:
- Basic types: str, int, float, bool
- Complex types: List[T], Dict[K, V], Optional[T], Union[A, B], Tuple, Set
- Pydantic models: BaseModel subclasses
- Dataclasses: @dataclass decorated classes
- Custom types: UUID, datetime, Decimal, etc.

This module analyzes Python function signatures and type hints
to generate validation rules for the Rust framework.
"""

from typing import (
    Any, Callable, Dict, List, Optional, Type, Union, Tuple,
    get_type_hints, get_origin, get_args, Annotated, ForwardRef,
    Literal
)
import inspect
import sys
from dataclasses import dataclass, fields as dataclass_fields, is_dataclass
from enum import Enum

from .types import Path, Query, Body, Header, Depends

# Try to import Pydantic
try:
    from pydantic import BaseModel
    from pydantic.fields import FieldInfo
    HAS_PYDANTIC = True
except ImportError:
    BaseModel = None
    FieldInfo = None
    HAS_PYDANTIC = False


def extract_handler_meta(
    handler: Callable,
    method: str,
    path: str,
) -> Dict[str, Any]:
    """Extract comprehensive metadata from a handler function.

    This function analyzes:
    1. Function signature and type hints
    2. Annotated type markers (Path, Query, Body, Header, Depends)
    3. Pydantic models and dataclasses
    4. Nested object structures

    Args:
        handler: The handler function
        method: HTTP method (GET, POST, etc.)
        path: URL path pattern

    Returns:
        Dictionary with validator configuration, dependencies, and response schema
    """
    sig = inspect.signature(handler)

    # Get type hints with forward reference resolution
    try:
        hints = get_type_hints(handler, include_extras=True)
    except Exception:
        hints = {}

    path_params: List[Dict] = []
    query_params: List[Dict] = []
    body_param: Optional[Dict] = None
    header_params: List[Dict] = []
    dependencies: List[Dict] = []

    # Extract path parameter names from path pattern
    import re
    path_param_names = set(re.findall(r'\{(\w+)\}', path))

    for param_name, param in sig.parameters.items():
        if param_name in ('self', 'cls', 'request', 'response'):
            continue

        hint = hints.get(param_name, Any)
        origin = get_origin(hint)

        # Extract marker from Annotated type
        marker = None
        base_type = hint
        if origin is Annotated:
            args = get_args(hint)
            base_type = args[0]
            for arg in args[1:]:
                if isinstance(arg, (Path, Query, Body, Header, Depends)):
                    marker = arg
                    break

        # Handle Depends (dependency injection)
        if isinstance(marker, Depends):
            dep_info = extract_dependency_info(param_name, marker, base_type)
            dependencies.append(dep_info)
            continue

        # Determine parameter location and extract info
        if param_name in path_param_names:
            # Path parameter
            info = extract_param_info(
                param_name,
                base_type,
                marker if isinstance(marker, Path) else Path(),
                "path"
            )
            path_params.append(info)

        elif isinstance(marker, Header):
            # Header parameter
            info = extract_param_info(param_name, base_type, marker, "header")
            info["alias"] = marker.alias or param_name.replace("_", "-").title()
            header_params.append(info)

        elif isinstance(marker, Body):
            # Body parameter
            body_param = extract_body_info(param_name, base_type, marker)

        else:
            # Query parameter (default)
            query_marker = marker if isinstance(marker, Query) else Query()
            info = extract_param_info(param_name, base_type, query_marker, "query")

            # Handle default values
            if param.default is not inspect.Parameter.empty:
                info["default"] = param.default
                info["required"] = False
            elif query_marker.default is not ...:
                info["default"] = query_marker.default
                info["required"] = False
            else:
                info["required"] = True

            query_params.append(info)

    # Extract return type for response schema
    return_type = hints.get('return', Any)
    response_schema = extract_type_schema(return_type)

    return {
        "validator": {
            "path_params": path_params,
            "query_params": query_params,
            "header_params": header_params,
            "body": body_param,
        },
        "dependencies": dependencies,
        "response_schema": response_schema,
    }


def extract_param_info(
    name: str,
    type_hint: Type,
    marker: Any,
    location: str,
) -> Dict[str, Any]:
    """Extract parameter information with full type and constraint details.

    Args:
        name: Parameter name
        type_hint: Python type hint
        marker: Parameter marker (Path, Query, etc.)
        location: Parameter location (path, query, header, body)

    Returns:
        Dictionary with parameter metadata
    """
    info: Dict[str, Any] = {
        "name": name,
        "location": location,
        "type": extract_type_schema(type_hint),
    }

    # Add constraints from marker
    if hasattr(marker, 'ge') and marker.ge is not None:
        info["type"]["minimum"] = marker.ge
    if hasattr(marker, 'gt') and marker.gt is not None:
        info["type"]["exclusive_minimum"] = marker.gt
    if hasattr(marker, 'le') and marker.le is not None:
        info["type"]["maximum"] = marker.le
    if hasattr(marker, 'lt') and marker.lt is not None:
        info["type"]["exclusive_maximum"] = marker.lt
    if hasattr(marker, 'min_length') and marker.min_length is not None:
        info["type"]["min_length"] = marker.min_length
    if hasattr(marker, 'max_length') and marker.max_length is not None:
        info["type"]["max_length"] = marker.max_length
    if hasattr(marker, 'pattern') and marker.pattern is not None:
        info["type"]["pattern"] = marker.pattern

    # Add metadata
    if hasattr(marker, 'description') and marker.description:
        info["description"] = marker.description
    if hasattr(marker, 'example') and marker.example is not None:
        info["example"] = marker.example
    if hasattr(marker, 'deprecated') and marker.deprecated:
        info["deprecated"] = True

    return info


def extract_body_info(
    name: str,
    type_hint: Type,
    marker: Body,
) -> Dict[str, Any]:
    """Extract request body information including nested object schema.

    Args:
        name: Parameter name
        type_hint: Python type hint
        marker: Body marker

    Returns:
        Dictionary with body metadata
    """
    schema = extract_type_schema(type_hint)

    return {
        "name": name,
        "location": "body",
        "type": schema,
        "media_type": marker.media_type,
        "description": marker.description or None,
        "example": marker.example,
        "embed": marker.embed,
    }


def extract_dependency_info(
    name: str,
    marker: Depends,
    type_hint: Type,
) -> Dict[str, Any]:
    """Extract dependency injection information.

    Args:
        name: Parameter name
        marker: Depends marker
        type_hint: Python type hint

    Returns:
        Dictionary with dependency metadata
    """
    dependency = marker.dependency

    # Get dependency's own parameters
    if callable(dependency):
        dep_meta = extract_handler_meta(dependency, "GET", "")
    else:
        dep_meta = {}

    return {
        "name": name,
        "dependency": dependency,
        "type": extract_type_schema(type_hint),
        "use_cache": marker.use_cache,
        "nested_params": dep_meta.get("validator", {}),
    }


def extract_type_schema(type_hint: Type) -> Dict[str, Any]:
    """Extract comprehensive type schema from a Python type hint.

    Returns a dictionary that can be used for:
    1. Rust validation (TypeDescriptor)
    2. OpenAPI schema generation
    3. JSON Schema generation

    Args:
        type_hint: Python type annotation

    Returns:
        Dictionary with type schema
    """
    if type_hint is None or type_hint is type(None):
        return {"type": "null"}

    origin = get_origin(type_hint)
    args = get_args(type_hint)

    # Handle Annotated (strip annotations)
    if origin is Annotated:
        return extract_type_schema(args[0])

    # Handle Optional[T] = Union[T, None]
    if origin is Union:
        non_none_args = [a for a in args if a is not type(None)]
        if len(non_none_args) == 1 and type(None) in args:
            # Optional[T]
            inner = extract_type_schema(non_none_args[0])
            return {"type": "optional", "inner": inner}
        else:
            # Union[A, B, ...]
            return {
                "type": "union",
                "variants": [extract_type_schema(a) for a in args if a is not type(None)],
                "nullable": type(None) in args,
            }

    # Handle List[T]
    if origin is list:
        item_type = args[0] if args else Any
        return {
            "type": "list",
            "items": extract_type_schema(item_type),
        }

    # Handle Tuple[T, ...]
    if origin is tuple:
        if args and args[-1] is ...:
            # Tuple[T, ...] = variable length tuple (like list)
            return {
                "type": "list",
                "items": extract_type_schema(args[0]),
            }
        else:
            # Fixed length tuple
            return {
                "type": "tuple",
                "items": [extract_type_schema(a) for a in args],
            }

    # Handle Dict[K, V]
    if origin is dict:
        key_type = args[0] if args else str
        value_type = args[1] if len(args) > 1 else Any
        return {
            "type": "object",
            "additional_properties": extract_type_schema(value_type),
        }

    # Handle Set[T]
    if origin is set or origin is frozenset:
        item_type = args[0] if args else Any
        return {
            "type": "set",
            "items": extract_type_schema(item_type),
        }

    # Handle Literal[...]
    if origin is Literal:
        return {
            "type": "literal",
            "values": list(args),
        }

    # Handle Pydantic models
    if HAS_PYDANTIC and isinstance(type_hint, type) and issubclass(type_hint, BaseModel):
        return extract_pydantic_schema(type_hint)

    # Handle dataclasses
    if is_dataclass(type_hint) and isinstance(type_hint, type):
        return extract_dataclass_schema(type_hint)

    # Handle Enum
    if isinstance(type_hint, type) and issubclass(type_hint, Enum):
        return {
            "type": "enum",
            "values": [e.value for e in type_hint],
            "names": [e.name for e in type_hint],
        }

    # Handle basic types
    if type_hint is str:
        return {"type": "string"}
    if type_hint is int:
        return {"type": "int"}
    if type_hint is float:
        return {"type": "float"}
    if type_hint is bool:
        return {"type": "bool"}
    if type_hint is bytes:
        return {"type": "bytes"}
    if type_hint is Any:
        return {"type": "any"}

    # Handle untyped built-in collections
    if type_hint is list:
        return {"type": "list", "items": {"type": "any"}}
    if type_hint is dict:
        return {"type": "object", "additional_properties": {"type": "any"}}
    if type_hint is set or type_hint is frozenset:
        return {"type": "set", "items": {"type": "any"}}
    if type_hint is tuple:
        return {"type": "list", "items": {"type": "any"}}

    # Handle special types by name
    type_name = getattr(type_hint, '__name__', str(type_hint))

    if type_name == 'UUID':
        return {"type": "uuid"}
    if type_name == 'datetime':
        return {"type": "datetime"}
    if type_name == 'date':
        return {"type": "date"}
    if type_name == 'time':
        return {"type": "time"}
    if type_name == 'timedelta':
        return {"type": "timedelta"}
    if type_name == 'Decimal':
        return {"type": "decimal"}
    if type_name == 'Path' or type_name == 'PurePath':
        return {"type": "string", "format": "path"}
    if type_name == 'EmailStr':
        return {"type": "string", "format": "email"}
    if type_name == 'HttpUrl' or type_name == 'AnyUrl':
        return {"type": "string", "format": "uri"}

    # Default: treat as object
    return {"type": "any", "python_type": type_name}


def extract_pydantic_schema(model: Type["BaseModel"]) -> Dict[str, Any]:
    """Extract schema from a Pydantic model.

    Args:
        model: Pydantic BaseModel subclass

    Returns:
        Dictionary with object schema
    """
    if not HAS_PYDANTIC:
        return {"type": "object", "class_name": model.__name__}

    fields_info = []

    for field_name, field in model.model_fields.items():
        field_schema = extract_type_schema(field.annotation)

        field_info = {
            "name": field_name,
            "type": field_schema,
            "required": field.is_required(),
        }

        # Add field constraints
        if field.default is not None and field.default is not ...:
            field_info["default"] = field.default
        if field.description:
            field_info["description"] = field.description

        # Add validation constraints if available
        if hasattr(field, 'ge') and field.ge is not None:
            field_info["type"]["minimum"] = field.ge
        if hasattr(field, 'le') and field.le is not None:
            field_info["type"]["maximum"] = field.le
        if hasattr(field, 'min_length') and field.min_length is not None:
            field_info["type"]["min_length"] = field.min_length
        if hasattr(field, 'max_length') and field.max_length is not None:
            field_info["type"]["max_length"] = field.max_length
        if hasattr(field, 'pattern') and field.pattern is not None:
            field_info["type"]["pattern"] = field.pattern

        fields_info.append(field_info)

    return {
        "type": "object",
        "class_name": model.__name__,
        "fields": fields_info,
    }


def extract_dataclass_schema(dc: Type) -> Dict[str, Any]:
    """Extract schema from a dataclass.

    Args:
        dc: Dataclass type

    Returns:
        Dictionary with object schema
    """
    fields_info = []

    try:
        hints = get_type_hints(dc)
    except Exception:
        hints = {}

    for field in dataclass_fields(dc):
        field_type = hints.get(field.name, Any)
        field_schema = extract_type_schema(field_type)

        field_info = {
            "name": field.name,
            "type": field_schema,
        }

        # Check if field has default
        from dataclasses import MISSING
        if field.default is not MISSING:
            field_info["default"] = field.default
            field_info["required"] = False
        elif field.default_factory is not MISSING:
            field_info["required"] = False
        else:
            field_info["required"] = True

        fields_info.append(field_info)

    return {
        "type": "object",
        "class_name": dc.__name__,
        "fields": fields_info,
    }


def schema_to_rust_type_descriptor(schema: Dict[str, Any]) -> Dict[str, Any]:
    """Convert Python type schema to Rust TypeDescriptor format.

    This format matches what the Rust validation engine expects.

    Args:
        schema: Python type schema

    Returns:
        Dictionary in Rust TypeDescriptor format
    """
    type_name = schema.get("type", "any")

    if type_name == "string":
        result = {"type": "string"}
        if "min_length" in schema:
            result["min_length"] = schema["min_length"]
        if "max_length" in schema:
            result["max_length"] = schema["max_length"]
        if "pattern" in schema:
            result["pattern"] = schema["pattern"]
        if schema.get("format") == "email":
            result["type"] = "email"
        if schema.get("format") == "uri":
            result["type"] = "url"
        return result

    if type_name == "int":
        result = {"type": "int"}
        if "minimum" in schema:
            result["minimum"] = schema["minimum"]
        if "maximum" in schema:
            result["maximum"] = schema["maximum"]
        if "exclusive_minimum" in schema:
            result["exclusive_minimum"] = schema["exclusive_minimum"]
        if "exclusive_maximum" in schema:
            result["exclusive_maximum"] = schema["exclusive_maximum"]
        return result

    if type_name == "float":
        result = {"type": "float"}
        if "minimum" in schema:
            result["minimum"] = schema["minimum"]
        if "maximum" in schema:
            result["maximum"] = schema["maximum"]
        return result

    if type_name == "bool":
        return {"type": "bool"}

    if type_name == "list":
        return {
            "type": "list",
            "items": schema_to_rust_type_descriptor(schema.get("items", {"type": "any"})),
        }

    if type_name == "object":
        if "fields" in schema:
            return {
                "type": "object",
                "fields": [
                    {
                        "name": f["name"],
                        "type": schema_to_rust_type_descriptor(f["type"]),
                        "required": f.get("required", True),
                    }
                    for f in schema["fields"]
                ],
            }
        return {"type": "object"}

    if type_name == "optional":
        return {
            "type": "optional",
            "inner": schema_to_rust_type_descriptor(schema.get("inner", {"type": "any"})),
        }

    if type_name in ("uuid", "email", "url", "datetime", "date", "time"):
        return {"type": type_name}

    return {"type": "any"}


# Backward compatibility aliases
extract_type_info = extract_type_schema
