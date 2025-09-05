from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Any, Self, TypeVar, overload

T = TypeVar("T")


@dataclass(slots=True, frozen=True)
class QueryExpression:
    """Base class for query expressions."""

    field: str
    operator: str
    value: Any

    def __and__(self, other: QueryExpression) -> CompoundExpression:
        return CompoundExpression("and", [self, other])

    def __or__(self, other: QueryExpression) -> CompoundExpression:
        return CompoundExpression("or", [self, other])

    def __invert__(self) -> CompoundExpression:
        return CompoundExpression("not", [self])


@dataclass(slots=True, frozen=True)
class CompoundExpression:
    """Compound expression for combining multiple query expressions."""

    operator: str  # 'and', 'or', 'not'
    expressions: list[QueryExpression | CompoundExpression]

    def __and__(self, other: QueryExpression | CompoundExpression) -> CompoundExpression:
        return CompoundExpression("and", [self, other])

    def __or__(self, other: QueryExpression | CompoundExpression) -> CompoundExpression:
        return CompoundExpression("or", [self, other])

    def __invert__(self) -> CompoundExpression:
        return CompoundExpression("not", [self])


@dataclass(slots=True, frozen=True)
class UpdateExpression:
    """Represents a single database update operation."""

    field: str
    operator: str  # Update operator like set, inc, push, etc.
    value: Any
    modifiers: dict[str, Any] | None = None  # For complex operations with additional parameters

    def __post_init__(self) -> None:
        """Validate the update expression."""
        if not self.field:
            raise ValueError("Field cannot be empty")
        
        if not self.operator:
            raise ValueError("Operator cannot be empty")


class Field[T]:
    """Base field descriptor with operator support for query building."""

    def __init__(
        self,
        default: T | None = None,
        *,
        default_factory: Callable[[], T] | None = None,
        required: bool = True,
        db_field: str | None = None,
        primary_key: bool = False,
        index: bool = False,
        unique: bool = False,
    ) -> None:
        if default is not None and default_factory is not None:
            raise ValueError("Cannot specify both default and default_factory")

        self.default = default
        self.default_factory = default_factory
        self.required = required
        self.db_field = db_field
        self.primary_key = primary_key
        self.index = index
        self.unique = unique
        self.name: str | None = None  # Set by metaclass
        self.type: type[T] | None = None  # Set by metaclass

    def __set_name__(self, owner: type[Any], name: str) -> None:
        self.name = name
        if self.db_field is None:
            self.db_field = name

    @overload
    def __get__(self, obj: None, objtype: type[Any]) -> Self: ...

    @overload
    def __get__(self, obj: Any, objtype: type[Any]) -> T: ...

    def __get__(self, obj: Any | None, objtype: type[Any]) -> Self | T:
        if obj is None:
            return self

        value = obj.__dict__.get(self.name)
        if value is None:
            if self.default_factory is not None:
                value = self.default_factory()
                obj.__dict__[self.name] = value
            else:
                value = self.default

        return value

    def __set__(self, obj: Any, value: T) -> None:
        if self.required and value is None:
            raise ValueError(f"Field {self.name} is required")
        obj.__dict__[self.name] = value

    def __eq__(self, other: Any) -> QueryExpression:  # type: ignore[override]
        return QueryExpression(self.db_field or self.name or "", "eq", other)

    def __ne__(self, other: Any) -> QueryExpression:  # type: ignore[override]
        return QueryExpression(self.db_field or self.name or "", "ne", other)

    def __lt__(self, other: Any) -> QueryExpression:
        return QueryExpression(self.db_field or self.name or "", "lt", other)

    def __le__(self, other: Any) -> QueryExpression:
        return QueryExpression(self.db_field or self.name or "", "lte", other)

    def __gt__(self, other: Any) -> QueryExpression:
        return QueryExpression(self.db_field or self.name or "", "gt", other)

    def __ge__(self, other: Any) -> QueryExpression:
        return QueryExpression(self.db_field or self.name or "", "gte", other)

    def in_(self, values: list[Any]) -> QueryExpression:
        """Check if field value is in a list of values."""
        return QueryExpression(self.db_field or self.name or "", "in", values)

    def not_in(self, values: list[Any]) -> QueryExpression:
        """Check if field value is not in a list of values."""
        return QueryExpression(self.db_field or self.name or "", "not_in", values)

    def contains(self, value: str) -> QueryExpression:
        """String contains operation."""
        return QueryExpression(self.db_field or self.name or "", "contains", value)

    def startswith(self, value: str) -> QueryExpression:
        """String starts with operation."""
        return QueryExpression(self.db_field or self.name or "", "startswith", value)

    def endswith(self, value: str) -> QueryExpression:
        """String ends with operation."""
        return QueryExpression(self.db_field or self.name or "", "endswith", value)

    def exists(self, value: bool = True) -> QueryExpression:
        """Check if field exists (for document databases)."""
        return QueryExpression(self.db_field or self.name or "", "exists", value)

    # Update operations
    def set(self, value: T) -> UpdateExpression:
        """Set field to a value."""
        return UpdateExpression(self.db_field or self.name or "", "set", value)
    
    def unset(self) -> UpdateExpression:
        """Remove field from document."""
        return UpdateExpression(self.db_field or self.name or "", "unset", "")
    
    def rename(self, new_name: str) -> UpdateExpression:
        """Rename field."""
        return UpdateExpression(self.db_field or self.name or "", "rename", new_name)


class IntField(Field[int]):
    """Integer field with numeric update operations."""

    def inc(self, value: int = 1) -> UpdateExpression:
        """Increment field by value."""
        return UpdateExpression(self.db_field or self.name or "", "inc", value)
    
    def mul(self, value: int) -> UpdateExpression:
        """Multiply field by value."""
        return UpdateExpression(self.db_field or self.name or "", "mul", value)
    
    def min(self, value: int) -> UpdateExpression:
        """Set field to minimum of current value and provided value."""
        return UpdateExpression(self.db_field or self.name or "", "min", value)
    
    def max(self, value: int) -> UpdateExpression:
        """Set field to maximum of current value and provided value."""
        return UpdateExpression(self.db_field or self.name or "", "max", value)


class FloatField(Field[float]):
    """Float field with numeric update operations."""

    def inc(self, value: float) -> UpdateExpression:
        """Increment field by value."""
        return UpdateExpression(self.db_field or self.name or "", "inc", value)
    
    def mul(self, value: float) -> UpdateExpression:
        """Multiply field by value."""
        return UpdateExpression(self.db_field or self.name or "", "mul", value)
    
    def min(self, value: float) -> UpdateExpression:
        """Set field to minimum of current value and provided value."""
        return UpdateExpression(self.db_field or self.name or "", "min", value)
    
    def max(self, value: float) -> UpdateExpression:
        """Set field to maximum of current value and provided value."""
        return UpdateExpression(self.db_field or self.name or "", "max", value)


class StringField(Field[str]):
    """String field with additional string-specific operations."""

    def __init__(
        self,
        default: str | None = None,
        *,
        max_length: int | None = None,
        min_length: int | None = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(default, **kwargs)
        self.max_length = max_length
        self.min_length = min_length

    def regex(self, pattern: str) -> QueryExpression:
        """Regex match operation."""
        return QueryExpression(self.db_field or self.name or "", "regex", pattern)


class BoolField(Field[bool]):
    """Boolean field with toggle operation."""

    def toggle(self) -> UpdateExpression:
        """Toggle boolean field value."""
        # This will require special handling in the translator
        # For MongoDB, this would use an aggregation pipeline or conditional logic
        return UpdateExpression(self.db_field or self.name or "", "toggle", None)


class ListField[T](Field[list[T]]):
    """List field for array/list values."""

    def __init__(
        self,
        item_type: type[T],
        default: list[T] | None = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(default or [], **kwargs)
        self.item_type = item_type

    def contains_all(self, values: list[T]) -> QueryExpression:
        """Check if list contains all specified values."""
        return QueryExpression(self.db_field or self.name or "", "contains_all", values)

    def contains_any(self, values: list[T]) -> QueryExpression:
        """Check if list contains any of the specified values."""
        return QueryExpression(self.db_field or self.name or "", "contains_any", values)

    # Array update operations
    def push(
        self, 
        value: T, 
        *, 
        position: int | None = None, 
        slice: int | None = None, 
        sort: int | None = None
    ) -> UpdateExpression:
        """Add element to array."""
        modifiers = {}
        if position is not None:
            modifiers["$position"] = position
        if slice is not None:
            modifiers["$slice"] = slice
        if sort is not None:
            modifiers["$sort"] = sort
        
        return UpdateExpression(
            self.db_field or self.name or "", 
            "push", 
            value,
            modifiers or None
        )
    
    def push_all(self, values: list[T]) -> UpdateExpression:
        """Add multiple elements to array."""
        modifiers = {"$each": True}  # Marker for translator
        return UpdateExpression(
            self.db_field or self.name or "", 
            "push", 
            values,
            modifiers
        )
    
    def pull(self, value: T) -> UpdateExpression:
        """Remove matching elements from array."""
        return UpdateExpression(self.db_field or self.name or "", "pull", value)
    
    def pull_all(self, values: list[T]) -> UpdateExpression:
        """Remove multiple matching elements from array."""
        return UpdateExpression(self.db_field or self.name or "", "pullAll", values)
    
    def add_to_set(self, value: T) -> UpdateExpression:
        """Add element to array only if it doesn't exist."""
        return UpdateExpression(self.db_field or self.name or "", "addToSet", value)
    
    def add_to_set_each(self, values: list[T]) -> UpdateExpression:
        """Add multiple unique elements to array."""
        modifiers = {"$each": True}  # Marker for translator
        return UpdateExpression(
            self.db_field or self.name or "", 
            "addToSet", 
            values,
            modifiers
        )
    
    def pop(self, position: int = 1) -> UpdateExpression:
        """Remove first (-1) or last (1) element from array."""
        if position not in [-1, 1]:
            raise ValueError("Position must be -1 (first) or 1 (last)")
        return UpdateExpression(self.db_field or self.name or "", "pop", position)


class DictField(Field[dict[str, Any]]):
    """Dictionary/object field for nested data."""

    def get_nested(self, path: str) -> NestedFieldProxy:
        """Access nested field for queries."""
        full_path = f"{self.db_field or self.name or ''}.{path}"
        return NestedFieldProxy(full_path)

    # Nested update operations
    def set_field(self, path: str, value: Any) -> UpdateExpression:
        """Set nested field value."""
        full_path = f"{self.db_field or self.name or ''}.{path}"
        return UpdateExpression(full_path, "set", value)
    
    def unset_field(self, path: str) -> UpdateExpression:
        """Remove nested field."""
        full_path = f"{self.db_field or self.name or ''}.{path}"
        return UpdateExpression(full_path, "unset", "")
    
    def inc_field(self, path: str, value: int | float) -> UpdateExpression:
        """Increment nested numeric field."""
        full_path = f"{self.db_field or self.name or ''}.{path}"
        return UpdateExpression(full_path, "inc", value)


class NestedFieldProxy:
    """Proxy for accessing nested fields in queries."""

    def __init__(self, path: str) -> None:
        self.path = path

    def __eq__(self, other: Any) -> QueryExpression:  # type: ignore[override]
        return QueryExpression(self.path, "eq", other)

    def __ne__(self, other: Any) -> QueryExpression:  # type: ignore[override]
        return QueryExpression(self.path, "ne", other)

    def __lt__(self, other: Any) -> QueryExpression:
        return QueryExpression(self.path, "lt", other)

    def __le__(self, other: Any) -> QueryExpression:
        return QueryExpression(self.path, "lte", other)

    def __gt__(self, other: Any) -> QueryExpression:
        return QueryExpression(self.path, "gt", other)

    def __ge__(self, other: Any) -> QueryExpression:
        return QueryExpression(self.path, "gte", other)
