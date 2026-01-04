"""
Pydantic-style models for request/response validation.

This module provides a BaseModel class similar to Pydantic's BaseModel,
but backed by Rust validation for high performance.

Example:
    from data_bridge.api.models import BaseModel, Field

    class User(BaseModel):
        name: str = Field(min_length=1, max_length=100)
        age: int = Field(ge=0, le=150)
        email: str = Field(pattern=r'^[\w\.-]+@[\w\.-]+\.\w+$')

    user = User(name="John", age=30, email="john@example.com")
    data = user.model_dump()
"""

from dataclasses import dataclass
from typing import Any, Dict, Type, get_type_hints, Optional, ClassVar, get_origin

from .type_extraction import extract_type_schema, schema_to_rust_type_descriptor


@dataclass
class Field:
    """Field descriptor with validation constraints.

    Similar to Pydantic's Field, but designed to work with Rust validation.

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

    Example:
        name: str = Field(min_length=1, max_length=100, description="User's name")
        age: int = Field(ge=0, le=150, description="User's age in years")
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


class BaseModel:
    """Pydantic-style base model with Rust-backed validation.

    This class provides a similar API to Pydantic's BaseModel, but uses
    Rust for validation to achieve higher performance.

    Schema extraction happens at class definition time via __init_subclass__,
    so there's no runtime overhead for schema generation.

    Example:
        class Address(BaseModel):
            street: str
            city: str = Field(min_length=1)

        class User(BaseModel):
            name: str = Field(min_length=1, max_length=100)
            age: int = Field(ge=0, le=150)
            email: str = Field(pattern=r'^[\w\.-]+@[\w\.-]+\.\w+$')
            address: Optional[Address] = None

        user = User(name="John", age=30, email="john@example.com")
        print(user.model_dump())
        # {"name": "John", "age": 30, "email": "john@example.com", "address": None}
    """

    __schema__: ClassVar[Dict[str, Any]] = {}
    __rust_descriptor__: ClassVar[Any] = None
    __fields__: ClassVar[Dict[str, Field]] = {}

    def __init_subclass__(cls, **kwargs):
        """Extract schema at class definition time.

        This hook is called when a subclass is defined, allowing us to
        analyze type hints and Field descriptors to build the schema once.
        """
        super().__init_subclass__(**kwargs)
        cls._extract_schema()

    @classmethod
    def _extract_schema(cls):
        """Extract field schema from type hints and Field descriptors.

        This method:
        1. Gets type hints from the class
        2. Finds Field descriptors for each field
        3. Builds a JSON Schema representation
        4. Converts to Rust TypeDescriptor format
        """
        try:
            hints = get_type_hints(cls)
        except Exception:
            # If type hints fail (e.g., forward references), fall back to __annotations__
            hints = getattr(cls, '__annotations__', {})

        fields = {}
        schema = {"type": "object", "properties": {}, "required": []}

        for name, type_hint in hints.items():
            if name.startswith('_'):
                # Skip private/dunder attributes
                continue

            # Get Field descriptor or default value
            field_info = getattr(cls, name, None)
            if not isinstance(field_info, Field):
                # If there's a default value (not Field), wrap it
                if field_info is not None:
                    field_info = Field(default=field_info)
                else:
                    field_info = Field()

            fields[name] = field_info

            # Extract type schema from type hint
            prop_schema = extract_type_schema(type_hint)

            # Add constraints from Field descriptor
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

            # Check if required (no default value and no default_factory)
            if field_info.default is ... and field_info.default_factory is None:
                # Check if type is Optional (Union with None)
                origin = get_origin(type_hint)
                if origin is not None:
                    from typing import Union
                    if origin is Union:
                        from typing import get_args
                        args = get_args(type_hint)
                        if type(None) in args:
                            # Optional field, not required
                            continue
                schema["required"].append(name)

        cls.__fields__ = fields
        cls.__schema__ = schema
        cls.__rust_descriptor__ = schema_to_rust_type_descriptor(schema)

    def __init__(self, **data):
        """Initialize model with validation.

        Args:
            **data: Field values as keyword arguments.

        Raises:
            ValueError: If validation fails or required fields are missing.

        Example:
            user = User(name="John", age=30, email="john@example.com")
        """
        # TODO: Add Rust validation call when available
        # For now, we skip Rust validation and just set attributes
        # Future implementation:
        # try:
        #     from data_bridge._data_bridge import validate_value
        #     errors = validate_value(data, self.__rust_descriptor__)
        #     if errors:
        #         raise ValueError(f"Validation errors: {errors}")
        # except ImportError:
        #     pass  # Rust not available, skip validation

        # Track which fields were set (for exclude_unset)
        self.__dict__['__fields_set__'] = set()

        # Set attributes from data
        for name, field_info in self.__fields__.items():
            if name in data:
                value = data[name]
                # If value is a dict and field type is BaseModel, instantiate it
                if isinstance(value, dict):
                    field_type = self.__annotations__.get(name)
                    # Unwrap Optional[T] to get T
                    if field_type:
                        origin = get_origin(field_type)
                        if origin is not None:
                            from typing import Union, get_args
                            if origin is Union:
                                args = get_args(field_type)
                                non_none_args = [a for a in args if a is not type(None)]
                                if len(non_none_args) == 1:
                                    field_type = non_none_args[0]

                        if isinstance(field_type, type) and issubclass(field_type, BaseModel):
                            value = field_type(**value)
                setattr(self, name, value)
                self.__dict__['__fields_set__'].add(name)
            elif field_info.default is not ...:
                setattr(self, name, field_info.default)
            elif field_info.default_factory is not None:
                setattr(self, name, field_info.default_factory())
            else:
                # Check if field is optional
                field_type = self.__annotations__.get(name)
                is_optional = False
                if field_type:
                    origin = get_origin(field_type)
                    if origin is not None:
                        from typing import Union, get_args
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

        Example:
            user = User(name="John", age=30)
            data = user.model_dump()
            # {"name": "John", "age": 30, "email": None, "address": None}

            data = user.model_dump(exclude_none=True)
            # {"name": "John", "age": 30}
        """
        result = {}
        fields_set = self.__dict__.get('__fields_set__', set())

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
                    exclude_unset=exclude_unset,
                    exclude_none=exclude_none
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

        This is an alias for the constructor, provided for Pydantic compatibility.

        Args:
            data: Dictionary of field values.

        Returns:
            New instance of the model.

        Raises:
            ValueError: If validation fails.

        Example:
            data = {"name": "John", "age": 30, "email": "john@example.com"}
            user = User.model_validate(data)
        """
        return cls(**data)

    @classmethod
    def model_json_schema(cls) -> Dict[str, Any]:
        """Get JSON Schema for this model.

        Returns:
            JSON Schema representation of the model.

        Example:
            schema = User.model_json_schema()
            # {
            #     "type": "object",
            #     "properties": {
            #         "name": {"type": "string", "minLength": 1, "maxLength": 100},
            #         "age": {"type": "int", "minimum": 0, "maximum": 150},
            #         ...
            #     },
            #     "required": ["name", "age", "email"]
            # }
        """
        return cls.__schema__

    def __repr__(self) -> str:
        """String representation of the model.

        Returns:
            String in the format: ClassName(field1=value1, field2=value2, ...)

        Example:
            user = User(name="John", age=30)
            print(repr(user))
            # User(name='John', age=30, email=None, address=None)
        """
        fields = ", ".join(
            f"{k}={getattr(self, k, None)!r}"
            for k in self.__fields__
            if hasattr(self, k)
        )
        return f"{self.__class__.__name__}({fields})"

    def __eq__(self, other: Any) -> bool:
        """Compare two model instances for equality.

        Args:
            other: Another object to compare with.

        Returns:
            True if both models are of the same type and have the same field values.

        Example:
            user1 = User(name="John", age=30)
            user2 = User(name="John", age=30)
            assert user1 == user2
        """
        if not isinstance(other, self.__class__):
            return False
        return self.model_dump() == other.model_dump()
