"""
Tests for Pydantic-style BaseModel implementation.

These tests verify:
- Basic model creation and attribute access
- Field constraints and validation
- Required vs optional fields
- Nested models
- model_dump() method with various options
- model_validate() method
- model_json_schema() method
- Equality comparison
"""

import pytest
from ouroboros.test import expect
from typing import Optional, List
from ouroboros.api import BaseModel, Field


class TestBasicModel:
    """Test basic model functionality."""

    def test_simple_model_creation(self):
        """Test creating a simple model with basic types."""

        class User(BaseModel):
            name: str
            age: int

        user = User(name="John", age=30)
        assert user.name == "John"
        assert user.age == 30

    def test_model_with_defaults(self):
        """Test model with default values."""

        class User(BaseModel):
            name: str
            age: int = 18
            active: bool = True

        user = User(name="Jane")
        assert user.name == "Jane"
        assert user.age == 18
        assert user.active is True

    def test_model_with_optional_fields(self):
        """Test model with Optional fields."""

        class User(BaseModel):
            name: str
            email: Optional[str] = None
            phone: Optional[str] = None

        user = User(name="John")
        assert user.name == "John"
        assert user.email is None
        assert user.phone is None

        user2 = User(name="Jane", email="jane@example.com")
        assert user2.email == "jane@example.com"

    def test_missing_required_field_raises_error(self):
        """Test that missing required fields raise ValueError."""

        class User(BaseModel):
            name: str
            age: int

        expect(lambda: User(name="John")).to_raise(ValueError)

        expect(lambda: User(age=30)).to_raise(ValueError)


class TestFieldConstraints:
    """Test Field constraints."""

    def test_field_with_description(self):
        """Test Field with description metadata."""

        class User(BaseModel):
            name: str = Field(description="User's full name")
            age: int = Field(ge=0, le=150, description="Age in years")

        schema = User.model_json_schema()
        assert schema["properties"]["name"]["description"] == "User's full name"
        assert schema["properties"]["age"]["description"] == "Age in years"

    def test_numeric_constraints_in_schema(self):
        """Test that numeric constraints appear in schema."""

        class Product(BaseModel):
            price: float = Field(ge=0, le=10000)
            quantity: int = Field(gt=0, lt=1000000)
            discount: float = Field(ge=0, le=1, multiple_of=0.01)

        schema = Product.model_json_schema()
        assert schema["properties"]["price"]["minimum"] == 0
        assert schema["properties"]["price"]["maximum"] == 10000
        assert schema["properties"]["quantity"]["exclusiveMinimum"] == 0
        assert schema["properties"]["quantity"]["exclusiveMaximum"] == 1000000
        assert schema["properties"]["discount"]["multipleOf"] == 0.01

    def test_string_constraints_in_schema(self):
        """Test that string constraints appear in schema."""

        class User(BaseModel):
            name: str = Field(min_length=1, max_length=100)
            email: str = Field(pattern=r'^[\w\.-]+@[\w\.-]+\.\w+$')
            bio: str = Field(max_length=500)

        schema = User.model_json_schema()
        assert schema["properties"]["name"]["minLength"] == 1
        assert schema["properties"]["name"]["maxLength"] == 100
        assert schema["properties"]["email"]["pattern"] == r'^[\w\.-]+@[\w\.-]+\.\w+$'
        assert schema["properties"]["bio"]["maxLength"] == 500

    def test_collection_constraints_in_schema(self):
        """Test that collection constraints appear in schema."""

        class Team(BaseModel):
            members: List[str] = Field(min_items=1, max_items=10)
            tags: List[str] = Field(min_items=0, max_items=20)

        schema = Team.model_json_schema()
        assert schema["properties"]["members"]["minItems"] == 1
        assert schema["properties"]["members"]["maxItems"] == 10
        assert schema["properties"]["tags"]["maxItems"] == 20

    def test_field_with_example(self):
        """Test Field with example value."""

        class User(BaseModel):
            email: str = Field(example="user@example.com")
            age: int = Field(example=25)

        schema = User.model_json_schema()
        assert schema["properties"]["email"]["example"] == "user@example.com"
        assert schema["properties"]["age"]["example"] == 25

    def test_field_with_title(self):
        """Test Field with title."""

        class User(BaseModel):
            full_name: str = Field(title="Full Name")

        schema = User.model_json_schema()
        assert schema["properties"]["full_name"]["title"] == "Full Name"


class TestNestedModels:
    """Test nested model functionality."""

    def test_nested_model(self):
        """Test model with nested BaseModel."""

        class Address(BaseModel):
            street: str
            city: str
            country: str = "USA"

        class User(BaseModel):
            name: str
            address: Address

        user = User(
            name="John",
            address=Address(street="123 Main St", city="New York")
        )
        assert user.name == "John"
        assert user.address.street == "123 Main St"
        assert user.address.city == "New York"
        assert user.address.country == "USA"

    def test_nested_model_from_dict(self):
        """Test creating nested model from dictionary."""

        class Address(BaseModel):
            street: str
            city: str

        class User(BaseModel):
            name: str
            address: Address

        user = User(
            name="John",
            address={"street": "123 Main St", "city": "New York"}
        )
        assert user.name == "John"
        assert isinstance(user.address, Address)
        assert user.address.street == "123 Main St"
        assert user.address.city == "New York"

    def test_optional_nested_model(self):
        """Test optional nested model."""

        class Address(BaseModel):
            street: str
            city: str

        class User(BaseModel):
            name: str
            address: Optional[Address] = None

        user1 = User(name="John")
        assert user1.address is None

        user2 = User(
            name="Jane",
            address=Address(street="456 Oak Ave", city="Boston")
        )
        assert user2.address is not None
        assert user2.address.street == "456 Oak Ave"


class TestModelDump:
    """Test model_dump() method."""

    def test_basic_model_dump(self):
        """Test basic model dump."""

        class User(BaseModel):
            name: str
            age: int
            active: bool = True

        user = User(name="John", age=30)
        data = user.model_dump()

        assert data == {"name": "John", "age": 30, "active": True}

    def test_model_dump_with_none(self):
        """Test model dump with None values."""

        class User(BaseModel):
            name: str
            email: Optional[str] = None

        user = User(name="John")
        data = user.model_dump()
        assert data == {"name": "John", "email": None}

    def test_model_dump_exclude_none(self):
        """Test model dump with exclude_none=True."""

        class User(BaseModel):
            name: str
            email: Optional[str] = None
            phone: Optional[str] = None

        user = User(name="John", email="john@example.com")
        data = user.model_dump(exclude_none=True)
        assert data == {"name": "John", "email": "john@example.com"}
        assert "phone" not in data

    def test_model_dump_exclude_unset(self):
        """Test model dump with exclude_unset=True."""

        class User(BaseModel):
            name: str
            age: int = 18
            active: bool = True

        user = User(name="John")
        data = user.model_dump(exclude_unset=True)
        assert data == {"name": "John"}
        assert "age" not in data
        assert "active" not in data

    def test_nested_model_dump(self):
        """Test dumping nested models."""

        class Address(BaseModel):
            street: str
            city: str

        class User(BaseModel):
            name: str
            address: Address

        user = User(
            name="John",
            address=Address(street="123 Main St", city="New York")
        )
        data = user.model_dump()

        assert data == {
            "name": "John",
            "address": {
                "street": "123 Main St",
                "city": "New York"
            }
        }

    def test_list_of_models_dump(self):
        """Test dumping list of models."""

        class Tag(BaseModel):
            name: str
            priority: int = 0

        class Article(BaseModel):
            title: str
            tags: List[Tag]

        article = Article(
            title="Test Article",
            tags=[
                Tag(name="python", priority=1),
                Tag(name="rust")
            ]
        )
        data = article.model_dump()

        assert data == {
            "title": "Test Article",
            "tags": [
                {"name": "python", "priority": 1},
                {"name": "rust", "priority": 0}
            ]
        }


class TestModelValidate:
    """Test model_validate() class method."""

    def test_model_validate(self):
        """Test creating model from dict via model_validate()."""

        class User(BaseModel):
            name: str
            age: int

        data = {"name": "John", "age": 30}
        user = User.model_validate(data)

        assert user.name == "John"
        assert user.age == 30

    def test_model_validate_with_nested(self):
        """Test model_validate with nested models."""

        class Address(BaseModel):
            street: str
            city: str

        class User(BaseModel):
            name: str
            address: Address

        data = {
            "name": "John",
            "address": {
                "street": "123 Main St",
                "city": "New York"
            }
        }
        user = User.model_validate(data)

        assert user.name == "John"
        assert isinstance(user.address, Address)
        assert user.address.street == "123 Main St"


class TestModelJsonSchema:
    """Test model_json_schema() class method."""

    def test_model_json_schema(self):
        """Test getting JSON schema from model."""

        class User(BaseModel):
            name: str = Field(min_length=1)
            age: int = Field(ge=0, le=150)
            email: Optional[str] = None

        schema = User.model_json_schema()

        assert schema["type"] == "object"
        assert "properties" in schema
        assert "name" in schema["properties"]
        assert "age" in schema["properties"]
        assert "email" in schema["properties"]
        assert "name" in schema["required"]
        assert "age" in schema["required"]
        # email is optional, so not in required

    def test_nested_model_schema(self):
        """Test schema for nested models."""

        class Address(BaseModel):
            street: str
            city: str

        class User(BaseModel):
            name: str
            address: Address

        schema = User.model_json_schema()

        assert "address" in schema["properties"]
        address_schema = schema["properties"]["address"]
        assert address_schema["type"] == "object"
        assert "fields" in address_schema
        assert len(address_schema["fields"]) == 2


class TestModelEquality:
    """Test model equality comparison."""

    def test_equal_models(self):
        """Test that models with same values are equal."""

        class User(BaseModel):
            name: str
            age: int

        user1 = User(name="John", age=30)
        user2 = User(name="John", age=30)

        assert user1 == user2

    def test_unequal_models(self):
        """Test that models with different values are not equal."""

        class User(BaseModel):
            name: str
            age: int

        user1 = User(name="John", age=30)
        user2 = User(name="Jane", age=25)

        assert user1 != user2

    def test_different_types_not_equal(self):
        """Test that different model types are not equal."""

        class User(BaseModel):
            name: str

        class Admin(BaseModel):
            name: str

        user = User(name="John")
        admin = Admin(name="John")

        assert user != admin


class TestModelRepr:
    """Test model __repr__ method."""

    def test_model_repr(self):
        """Test string representation of model."""

        class User(BaseModel):
            name: str
            age: int

        user = User(name="John", age=30)
        repr_str = repr(user)

        assert "User" in repr_str
        assert "name='John'" in repr_str
        assert "age=30" in repr_str


class TestComplexExample:
    """Test complex real-world example."""

    def test_complex_user_model(self):
        """Test a complex user model with multiple features."""

        class Address(BaseModel):
            street: str = Field(min_length=1)
            city: str = Field(min_length=1)
            zip_code: str = Field(pattern=r'^\d{5}(-\d{4})?$')
            country: str = "USA"

        class User(BaseModel):
            name: str = Field(min_length=1, max_length=100)
            age: int = Field(ge=0, le=150)
            email: str = Field(pattern=r'^[\w\.-]+@[\w\.-]+\.\w+$')
            address: Optional[Address] = None
            tags: List[str] = Field(default_factory=list)
            active: bool = True

        # Create user with all fields
        user = User(
            name="John Doe",
            age=30,
            email="john@example.com",
            address=Address(
                street="123 Main St",
                city="New York",
                zip_code="10001"
            ),
            tags=["developer", "python"]
        )

        assert user.name == "John Doe"
        assert user.age == 30
        assert user.email == "john@example.com"
        assert user.address.street == "123 Main St"
        assert user.address.country == "USA"
        assert user.tags == ["developer", "python"]
        assert user.active is True

        # Test dump
        data = user.model_dump()
        assert data["name"] == "John Doe"
        assert data["address"]["city"] == "New York"

        # Test dump with exclude_none
        user2 = User(name="Jane", age=25, email="jane@example.com")
        data2 = user2.model_dump(exclude_none=True)
        assert "address" not in data2

        # Test schema
        schema = User.model_json_schema()
        assert "name" in schema["properties"]
        assert schema["properties"]["name"]["minLength"] == 1
        assert schema["properties"]["age"]["minimum"] == 0


class TestDefaultFactory:
    """Test default_factory functionality."""

    def test_default_factory(self):
        """Test Field with default_factory."""

        class User(BaseModel):
            name: str
            tags: List[str] = Field(default_factory=list)
            metadata: dict = Field(default_factory=dict)

        user1 = User(name="John")
        user2 = User(name="Jane")

        # Each instance should get its own list/dict
        user1.tags.append("python")
        assert user1.tags == ["python"]
        assert user2.tags == []

        user1.metadata["key"] = "value"
        assert user1.metadata == {"key": "value"}
        assert user2.metadata == {}
