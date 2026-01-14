"""
Pydantic-style models for validation with Rust backend.

This module provides a BaseModel class similar to Pydantic's BaseModel,
but backed by Rust validation for high performance. Supports both
traditional and Annotated syntax.

Example (Traditional):
    class User(BaseModel):
        name: str = Field(min_length=1, max_length=100)
        age: int = Field(ge=0, le=150)

Example (Annotated - Pydantic v2 style):
    from typing import Annotated

    class User(BaseModel):
        name: Annotated[str, Field(min_length=1, max_length=100)]
        age: Annotated[int, Field(ge=0, le=150)]
"""

from dataclasses import dataclass, field as dataclass_field
from typing import (
    Any,
    Dict,
    Type,
    get_type_hints,
    Optional,
    ClassVar,
    get_origin,
    get_args,
    Union,
    List,
)

# Try to import Annotated from typing (Python 3.9+) or typing_extensions
try:
    from typing import Annotated
except ImportError:
    from typing_extensions import Annotated  # type: ignore


class ValidationError(Exception):
    """Validation error with structured error details.

    Attributes:
        errors: List of error dictionaries with 'loc', 'msg', and 'type' keys.
    """

    def __init__(self, errors: List[Dict[str, Any]], message: str = "Validation failed"):
        self.errors = errors
        super().__init__(message)

    def __str__(self) -> str:
        error_msgs = []
        for err in self.errors:
            loc = " -> ".join(str(l) for l in err.get("loc", []))
            msg = err.get("msg", "Unknown error")
            error_msgs.append(f"{loc}: {msg}")
        return f"Validation failed: {'; '.join(error_msgs)}"


@dataclass
class Field:
    """Field descriptor with validation constraints.

    Supports both traditional assignment and Annotated syntax:

    Traditional:
        name: str = Field(min_length=1)

    Annotated:
        name: Annotated[str, Field(min_length=1)]

    Args:
        default: Default value for the field. Use ... for required fields.
        default_factory: Factory function to generate default value.
        description: Human-readable description of the field.
        ge: Greater than or equal to (numeric fields).
        gt: Greater than (numeric fields).
        le: Less than or equal to (numeric fields).
        lt: Less than (numeric fields).
        multiple_of: Value must be a multiple of this number.
        min_length: Minimum length for strings or collections.
        max_length: Maximum length for strings or collections.
        pattern: Regex pattern for string validation.
        min_items: Minimum number of items in collection.
        max_items: Maximum number of items in collection.
        example: Example value for documentation.
        title: Human-readable title for the field.
    """

    default: Any = ...  # ... means required
    default_factory: Any = None
    description: str = ""
    # Numeric constraints
    ge: Optional[float] = None  # greater than or equal
    gt: Optional[float] = None  # greater than
    le: Optional[float] = None  # less than or equal
    lt: Optional[float] = None  # less than
    multiple_of: Optional[float] = None
    # String constraints
    min_length: Optional[int] = None
    max_length: Optional[int] = None
    pattern: Optional[str] = None
    # Collection constraints
    min_items: Optional[int] = None
    max_items: Optional[int] = None
    # Metadata
    example: Any = None
    title: Optional[str] = None


def _get_rust_validate():
    """Get the Rust validate_value function if available."""
    try:
        from ouroboros.ouroboros.api import validate_value
        return validate_value
    except ImportError:
        return None


def _extract_annotated_field(type_hint: Any) -> tuple:
    """Extract Field from Annotated type hint.

    Args:
        type_hint: A type hint, possibly Annotated[T, Field(...)]

    Returns:
        Tuple of (base_type, field_info) where field_info is Field or None
    """
    origin = get_origin(type_hint)
    if origin is Annotated:
        args = get_args(type_hint)
        if len(args) >= 2:
            base_type = args[0]
            # Look for Field in the metadata
            for metadata in args[1:]:
                if isinstance(metadata, Field):
                    return base_type, metadata
            return base_type, None
    return type_hint, None


def _type_to_schema(type_hint: Any, seen_models: Optional[set] = None) -> Dict[str, Any]:
    """Convert Python type hint to JSON Schema.

    Args:
        type_hint: Python type hint
        seen_models: Set of already-seen model classes (for recursion detection)

    Returns:
        JSON Schema dictionary
    """
    if seen_models is None:
        seen_models = set()

    # Handle None/NoneType
    if type_hint is None or type_hint is type(None):
        return {"type": "null"}

    # Handle Annotated - extract base type
    origin = get_origin(type_hint)
    if origin is Annotated:
        args = get_args(type_hint)
        if args:
            return _type_to_schema(args[0], seen_models)

    # Handle Optional[T] = Union[T, None]
    if origin is Union:
        args = get_args(type_hint)
        non_none_args = [a for a in args if a is not type(None)]
        if len(non_none_args) == 1 and type(None) in args:
            # This is Optional[T]
            inner_schema = _type_to_schema(non_none_args[0], seen_models)
            return {"anyOf": [inner_schema, {"type": "null"}]}
        # General Union
        return {"anyOf": [_type_to_schema(a, seen_models) for a in args]}

    # Handle List[T]
    if origin is list or origin is List:
        args = get_args(type_hint)
        items_schema = {"type": "any"} if not args else _type_to_schema(args[0], seen_models)
        return {"type": "array", "items": items_schema}

    # Handle Dict[K, V]
    if origin is dict:
        args = get_args(type_hint)
        if len(args) == 2:
            value_schema = _type_to_schema(args[1], seen_models)
            return {"type": "object", "additionalProperties": value_schema}
        return {"type": "object"}

    # Handle nested BaseModel
    if isinstance(type_hint, type) and issubclass(type_hint, BaseModel):
        # Prevent infinite recursion
        if type_hint in seen_models:
            return {"type": "object"}
        seen_models.add(type_hint)
        # Return the nested model's schema
        return type_hint.__schema__.copy()

    # Basic types
    if type_hint is str:
        return {"type": "string"}
    if type_hint is int:
        return {"type": "integer"}
    if type_hint is float:
        return {"type": "number"}
    if type_hint is bool:
        return {"type": "boolean"}
    if type_hint is bytes:
        return {"type": "string", "format": "binary"}

    # Special types
    type_name = getattr(type_hint, "__name__", str(type_hint))
    if type_name == "UUID":
        return {"type": "uuid"}
    if type_name == "datetime":
        return {"type": "datetime"}
    if type_name == "date":
        return {"type": "date"}
    if type_name == "time":
        return {"type": "time"}
    if type_name == "Decimal":
        return {"type": "decimal"}

    # Fallback
    return {"type": "any"}


class BaseModel:
    """Pydantic-style base model with Rust-backed validation.

    Supports both traditional and Annotated syntax for field definitions.
    Schema extraction happens at class definition time for zero runtime overhead.

    Traditional syntax:
        class User(BaseModel):
            name: str = Field(min_length=1, max_length=100)
            age: int = Field(ge=0, le=150)

    Annotated syntax (Pydantic v2 style):
        class User(BaseModel):
            name: Annotated[str, Field(min_length=1, max_length=100)]
            age: Annotated[int, Field(ge=0, le=150)]

    Both styles can be mixed in the same model.
    """

    __schema__: ClassVar[Dict[str, Any]] = {}
    __rust_descriptor__: ClassVar[Any] = None
    __fields__: ClassVar[Dict[str, Field]] = {}
    __field_types__: ClassVar[Dict[str, Any]] = {}

    def __init_subclass__(cls, **kwargs):
        """Extract schema at class definition time."""
        super().__init_subclass__(**kwargs)
        cls._extract_schema()

    @classmethod
    def _extract_schema(cls):
        """Extract field schema from type hints and Field descriptors.

        Handles both traditional and Annotated syntax.
        """
        try:
            hints = get_type_hints(cls, include_extras=True)
        except Exception:
            hints = getattr(cls, "__annotations__", {})

        fields: Dict[str, Field] = {}
        field_types: Dict[str, Any] = {}
        schema: Dict[str, Any] = {"type": "object", "properties": {}, "required": []}

        for name, type_hint in hints.items():
            if name.startswith("_"):
                continue

            # Extract Field from Annotated if present
            base_type, annotated_field = _extract_annotated_field(type_hint)

            # Get Field descriptor from class attribute
            class_attr = getattr(cls, name, None)
            class_field = class_attr if isinstance(class_attr, Field) else None

            # Determine which Field to use (Annotated takes precedence)
            if annotated_field is not None:
                field_info = annotated_field
                # If there's also a class-level default, merge it
                if class_attr is not None and not isinstance(class_attr, Field):
                    if field_info.default is ...:
                        field_info = Field(
                            default=class_attr,
                            description=field_info.description,
                            ge=field_info.ge,
                            gt=field_info.gt,
                            le=field_info.le,
                            lt=field_info.lt,
                            multiple_of=field_info.multiple_of,
                            min_length=field_info.min_length,
                            max_length=field_info.max_length,
                            pattern=field_info.pattern,
                            min_items=field_info.min_items,
                            max_items=field_info.max_items,
                            example=field_info.example,
                            title=field_info.title,
                        )
            elif class_field is not None:
                field_info = class_field
            elif class_attr is not None:
                # Has a default value but no Field
                field_info = Field(default=class_attr)
            else:
                # No Field, no default - required field
                field_info = Field()

            fields[name] = field_info
            field_types[name] = base_type

            # Build property schema
            prop_schema = _type_to_schema(base_type)

            # Add constraints from Field
            if field_info.ge is not None:
                prop_schema["minimum"] = field_info.ge
            if field_info.gt is not None:
                prop_schema["exclusiveMinimum"] = field_info.gt
            if field_info.le is not None:
                prop_schema["maximum"] = field_info.le
            if field_info.lt is not None:
                prop_schema["exclusiveMaximum"] = field_info.lt
            if field_info.multiple_of is not None:
                prop_schema["multipleOf"] = field_info.multiple_of
            if field_info.min_length is not None:
                prop_schema["minLength"] = field_info.min_length
            if field_info.max_length is not None:
                prop_schema["maxLength"] = field_info.max_length
            if field_info.pattern is not None:
                prop_schema["pattern"] = field_info.pattern
            if field_info.min_items is not None:
                prop_schema["minItems"] = field_info.min_items
            if field_info.max_items is not None:
                prop_schema["maxItems"] = field_info.max_items
            if field_info.description:
                prop_schema["description"] = field_info.description
            if field_info.title:
                prop_schema["title"] = field_info.title
            if field_info.example is not None:
                prop_schema["example"] = field_info.example

            schema["properties"][name] = prop_schema

            # Check if required
            if field_info.default is ... and field_info.default_factory is None:
                # Check if type is Optional
                origin = get_origin(base_type)
                if origin is Union:
                    args = get_args(base_type)
                    if type(None) in args:
                        continue  # Optional, not required
                schema["required"].append(name)

        cls.__fields__ = fields
        cls.__field_types__ = field_types
        cls.__schema__ = schema
        cls.__rust_descriptor__ = schema

    def __init__(self, **data: Any):
        """Initialize model with validation.

        Args:
            **data: Field values as keyword arguments.

        Raises:
            ValidationError: If validation fails.
            ValueError: If required fields are missing.
        """
        # Run Rust validation if available
        validate_fn = _get_rust_validate()
        if validate_fn is not None and self.__rust_descriptor__:
            try:
                # Convert nested models to dicts for validation
                validation_data = {}
                for k, v in data.items():
                    if isinstance(v, BaseModel):
                        validation_data[k] = v.model_dump()
                    elif isinstance(v, list):
                        validation_data[k] = [
                            item.model_dump() if isinstance(item, BaseModel) else item
                            for item in v
                        ]
                    else:
                        validation_data[k] = v

                validate_fn(validation_data, self.__rust_descriptor__)
            except ValueError as e:
                # Parse error message and raise ValidationError
                raise ValidationError([], str(e))

        # Track which fields were set
        self.__dict__["__fields_set__"] = set()

        # Set attributes from data
        for name, field_info in self.__fields__.items():
            if name in data:
                value = data[name]
                # If value is a dict and field type is BaseModel, instantiate it
                if isinstance(value, dict):
                    field_type = self.__field_types__.get(name)
                    if field_type:
                        # Unwrap Optional[T]
                        origin = get_origin(field_type)
                        if origin is Union:
                            args = get_args(field_type)
                            non_none = [a for a in args if a is not type(None)]
                            if len(non_none) == 1:
                                field_type = non_none[0]

                        if isinstance(field_type, type) and issubclass(field_type, BaseModel):
                            value = field_type(**value)
                elif isinstance(value, list):
                    # Handle list of nested models
                    field_type = self.__field_types__.get(name)
                    if field_type:
                        origin = get_origin(field_type)
                        if origin is list or origin is List:
                            item_args = get_args(field_type)
                            if item_args:
                                item_type = item_args[0]
                                if isinstance(item_type, type) and issubclass(item_type, BaseModel):
                                    value = [
                                        item_type(**item) if isinstance(item, dict) else item
                                        for item in value
                                    ]

                setattr(self, name, value)
                self.__dict__["__fields_set__"].add(name)
            elif field_info.default is not ...:
                setattr(self, name, field_info.default)
            elif field_info.default_factory is not None:
                setattr(self, name, field_info.default_factory())
            else:
                # Check if optional
                field_type = self.__field_types__.get(name)
                is_optional = False
                if field_type:
                    origin = get_origin(field_type)
                    if origin is Union:
                        args = get_args(field_type)
                        if type(None) in args:
                            is_optional = True
                            setattr(self, name, None)

                if not is_optional:
                    raise ValueError(f"Missing required field: {name}")

    def model_dump(
        self,
        exclude_unset: bool = False,
        exclude_none: bool = False,
    ) -> Dict[str, Any]:
        """Convert model to dictionary.

        Args:
            exclude_unset: If True, exclude fields that were not explicitly set.
            exclude_none: If True, exclude fields with None values.

        Returns:
            Dictionary representation of the model.
        """
        result = {}
        fields_set = self.__dict__.get("__fields_set__", set())

        for name in self.__fields__:
            if not hasattr(self, name):
                continue

            if exclude_unset and name not in fields_set:
                continue

            value = getattr(self, name)

            if exclude_none and value is None:
                continue

            # Recursively dump nested models
            if isinstance(value, BaseModel):
                value = value.model_dump(
                    exclude_unset=exclude_unset, exclude_none=exclude_none
                )
            elif isinstance(value, list):
                value = [
                    v.model_dump(exclude_unset=exclude_unset, exclude_none=exclude_none)
                    if isinstance(v, BaseModel)
                    else v
                    for v in value
                ]
            elif isinstance(value, dict):
                value = {
                    k: v.model_dump(exclude_unset=exclude_unset, exclude_none=exclude_none)
                    if isinstance(v, BaseModel)
                    else v
                    for k, v in value.items()
                }

            result[name] = value

        return result

    @classmethod
    def model_validate(cls, data: Dict[str, Any]) -> "BaseModel":
        """Validate data and create model instance.

        Args:
            data: Dictionary of field values.

        Returns:
            New instance of the model.

        Raises:
            ValidationError: If validation fails.
        """
        return cls(**data)

    @classmethod
    def model_json_schema(cls) -> Dict[str, Any]:
        """Get JSON Schema for this model.

        Returns:
            JSON Schema representation of the model.
        """
        return cls.__schema__

    def __repr__(self) -> str:
        """String representation of the model."""
        fields = ", ".join(
            f"{k}={getattr(self, k, None)!r}"
            for k in self.__fields__
            if hasattr(self, k)
        )
        return f"{self.__class__.__name__}({fields})"

    def __eq__(self, other: Any) -> bool:
        """Compare two model instances for equality."""
        if not isinstance(other, self.__class__):
            return False
        return self.model_dump() == other.model_dump()

    def __hash__(self) -> int:
        """Hash the model based on its field values."""
        return hash(tuple(sorted(self.model_dump().items())))
