"""
Integration tests for BaseModel with API handlers.

These tests verify that BaseModel works correctly with FastAPI-style
route handlers and request/response serialization.
"""

import pytest
from typing import Optional
from data_bridge.api import App, BaseModel, Field, Body
from typing import Annotated


class CreateUserRequest(BaseModel):
    """Request model for creating a user."""
    name: str = Field(min_length=1, max_length=100)
    age: int = Field(ge=0, le=150)
    email: str


class UserResponse(BaseModel):
    """Response model for user data."""
    id: int
    name: str
    age: int
    email: str
    active: bool = True


class UpdateUserRequest(BaseModel):
    """Request model for updating a user."""
    name: Optional[str] = None
    age: Optional[int] = Field(None, ge=0, le=150)
    email: Optional[str] = None


class TestBaseModelIntegration:
    """Test BaseModel integration with API."""

    def test_model_schema_extraction(self):
        """Test that model schemas are correctly extracted."""
        schema = CreateUserRequest.model_json_schema()

        assert schema["type"] == "object"
        assert "name" in schema["properties"]
        assert "age" in schema["properties"]
        assert "email" in schema["properties"]

        # Check constraints
        assert schema["properties"]["name"]["minLength"] == 1
        assert schema["properties"]["name"]["maxLength"] == 100
        assert schema["properties"]["age"]["minimum"] == 0
        assert schema["properties"]["age"]["maximum"] == 150

        # Check required fields
        assert "name" in schema["required"]
        assert "age" in schema["required"]
        assert "email" in schema["required"]

    def test_request_model_creation(self):
        """Test creating request model from dictionary."""
        data = {
            "name": "John Doe",
            "age": 30,
            "email": "john@example.com"
        }
        request = CreateUserRequest.model_validate(data)

        assert request.name == "John Doe"
        assert request.age == 30
        assert request.email == "john@example.com"

    def test_response_model_serialization(self):
        """Test serializing response model to dictionary."""
        response = UserResponse(
            id=123,
            name="Jane Doe",
            age=25,
            email="jane@example.com"
        )

        data = response.model_dump()
        assert data == {
            "id": 123,
            "name": "Jane Doe",
            "age": 25,
            "email": "jane@example.com",
            "active": True
        }

    def test_optional_fields_in_update(self):
        """Test update model with optional fields."""
        # Update only name
        update1 = UpdateUserRequest(name="New Name")
        data1 = update1.model_dump(exclude_none=True)
        assert data1 == {"name": "New Name"}

        # Update only age
        update2 = UpdateUserRequest(age=35)
        data2 = update2.model_dump(exclude_none=True)
        assert data2 == {"age": 35}

        # Update all fields
        update3 = UpdateUserRequest(
            name="Updated Name",
            age=40,
            email="updated@example.com"
        )
        data3 = update3.model_dump(exclude_none=True)
        assert data3 == {
            "name": "Updated Name",
            "age": 40,
            "email": "updated@example.com"
        }

    def test_nested_models(self):
        """Test nested BaseModel structures."""

        class Address(BaseModel):
            street: str
            city: str
            country: str = "USA"

        class UserWithAddress(BaseModel):
            name: str
            address: Address

        # Create from nested dicts
        data = {
            "name": "John",
            "address": {
                "street": "123 Main St",
                "city": "New York"
            }
        }
        user = UserWithAddress.model_validate(data)

        assert user.name == "John"
        assert isinstance(user.address, Address)
        assert user.address.street == "123 Main St"
        assert user.address.city == "New York"
        assert user.address.country == "USA"

        # Serialize back to dict
        result = user.model_dump()
        assert result == {
            "name": "John",
            "address": {
                "street": "123 Main St",
                "city": "New York",
                "country": "USA"
            }
        }

    def test_model_validation_in_handler_context(self):
        """Test that models can be used in handler signatures."""

        # This simulates how models would be used in API handlers
        def create_user(request: CreateUserRequest) -> UserResponse:
            """Handler function using BaseModel."""
            # In real handler, this would save to database
            return UserResponse(
                id=1,
                name=request.name,
                age=request.age,
                email=request.email
            )

        # Simulate request
        request_data = {
            "name": "Test User",
            "age": 28,
            "email": "test@example.com"
        }
        request = CreateUserRequest.model_validate(request_data)

        # Call handler
        response = create_user(request)

        # Verify response
        assert response.id == 1
        assert response.name == "Test User"
        assert response.age == 28
        assert response.email == "test@example.com"
        assert response.active is True

        # Serialize response
        response_data = response.model_dump()
        assert response_data["id"] == 1
        assert response_data["name"] == "Test User"

    def test_field_constraints_preserved(self):
        """Test that Field constraints are preserved in schema."""

        class Product(BaseModel):
            name: str = Field(min_length=1, max_length=200, description="Product name")
            price: float = Field(ge=0, description="Price in USD")
            quantity: int = Field(ge=0, lt=1000000, description="Available quantity")

        schema = Product.model_json_schema()

        # Check name constraints
        assert schema["properties"]["name"]["minLength"] == 1
        assert schema["properties"]["name"]["maxLength"] == 200
        assert schema["properties"]["name"]["description"] == "Product name"

        # Check price constraints
        assert schema["properties"]["price"]["minimum"] == 0
        assert schema["properties"]["price"]["description"] == "Price in USD"

        # Check quantity constraints
        assert schema["properties"]["quantity"]["minimum"] == 0
        assert schema["properties"]["quantity"]["exclusiveMaximum"] == 1000000
        assert schema["properties"]["quantity"]["description"] == "Available quantity"
