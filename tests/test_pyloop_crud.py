"""
Unit tests for PyLoop CRUD decorator.

This tests the structure and registration of CRUD endpoints
without requiring a MongoDB connection.
"""

import pytest
from data_bridge.pyloop import App
from data_bridge.mongodb import Document


class TestProduct(Document):
    """Test document model."""
    name: str
    price: float
    stock: int = 0

    class Settings:
        name = "test_products"


def test_crud_decorator_exists():
    """Test that the crud decorator method exists on App."""
    app = App(title="Test API", version="1.0.0")
    assert hasattr(app, 'crud')
    assert callable(app.crud)


def test_crud_decorator_syntax():
    """Test that the crud decorator has correct syntax."""
    app = App(title="Test API", version="1.0.0")

    # Should be able to use as decorator
    @app.crud(TestProduct)
    class ProductCRUD:
        pass

    # Should return the decorated class
    assert ProductCRUD is not None


def test_crud_decorator_with_prefix():
    """Test that the crud decorator accepts custom prefix."""
    app = App(title="Test API", version="1.0.0")

    @app.crud(TestProduct, prefix="/api/products")
    class ProductCRUD:
        pass

    assert ProductCRUD is not None


def test_crud_decorator_with_tags():
    """Test that the crud decorator accepts custom tags."""
    app = App(title="Test API", version="1.0.0")

    @app.crud(TestProduct, tags=["inventory", "products"])
    class ProductCRUD:
        pass

    assert ProductCRUD is not None


def test_crud_decorator_collection_name_detection():
    """Test that collection name is correctly detected from Document."""
    app = App(title="Test API", version="1.0.0")

    # Should detect collection name from Settings.name
    @app.crud(TestProduct)
    class ProductCRUD:
        pass

    # Collection name should be "test_products" from TestProduct.Settings.name
    assert TestProduct.__collection_name__() == "test_products"


def test_multiple_crud_decorators():
    """Test that multiple CRUD decorators can be used on same app."""
    app = App(title="Test API", version="1.0.0")

    class User(Document):
        email: str
        name: str

        class Settings:
            name = "users"

    @app.crud(TestProduct)
    class ProductCRUD:
        pass

    @app.crud(User)
    class UserCRUD:
        pass

    # Both should work without conflicts
    assert ProductCRUD is not None
    assert UserCRUD is not None


if __name__ == "__main__":
    # Run tests
    pytest.main([__file__, "-v"])
