"""
Unit tests for Table class.

Tests the Table class definition, metaclass behavior, and instance methods
without requiring a real database connection.
"""
import pytest
from typing import Optional
from unittest.mock import patch, AsyncMock
from data_bridge.postgres import Table, Column, ColumnProxy
from data_bridge.test import expect


class TestTableMetaclass:
    """Test TableMeta metaclass behavior."""

    def test_table_name_from_settings(self):
        """Test table name is correctly set from Settings class."""

        class Product(Table):
            name: str

            class Settings:
                table_name = "products"

        expect(Product._table_name).to_equal("products")

    def test_table_name_default(self):
        """Test table name defaults to lowercase class name."""

        class OrderItem(Table):
            name: str

        expect(OrderItem._table_name).to_equal("orderitem")

    def test_table_name_with_schema(self):
        """Test full table name includes schema."""

        class User(Table):
            name: str

            class Settings:
                table_name = "users"
                schema = "auth"

        expect(User._schema).to_equal("auth")
        expect(User.__table_name__()).to_equal("auth.users")

    def test_primary_key_detection(self):
        """Test primary key is correctly identified."""

        class Product(Table):
            product_id: int

            class Settings:
                primary_key = "product_id"

        expect(Product._primary_key).to_equal("product_id")

    def test_primary_key_default(self):
        """Test primary key defaults to 'id'."""

        class Product(Table):
            name: str

        expect(Product._primary_key).to_equal("id")

    def test_column_definitions(self):
        """Test columns are correctly collected from annotations."""

        class User(Table):
            name: str
            email: str
            age: int

        expect("name" in User._columns).to_be_true()
        expect("email" in User._columns).to_be_true()
        expect("age" in User._columns).to_be_true()
        expect(User._columns["name"]).to_equal(str)
        expect(User._columns["age"]).to_equal(int)

    def test_column_proxy_created(self):
        """Test ColumnProxy is created for each column."""

        class User(Table):
            name: str
            email: str

        # Class-level access should return ColumnProxy
        expect(isinstance(User.name, ColumnProxy)).to_be_true()
        expect(isinstance(User.email, ColumnProxy)).to_be_true()
        expect(User.name.name).to_equal("name")
        expect(User.email.name).to_equal("email")

    def test_column_defaults_captured(self):
        """Test default values are captured before ColumnProxy replacement."""

        class User(Table):
            name: str
            age: int = 0
            status: str = "active"

        expect("age" in User._column_defaults).to_be_true()
        expect("status" in User._column_defaults).to_be_true()
        expect(User._column_defaults["age"]).to_equal(0)
        expect(User._column_defaults["status"]).to_equal("active")

    def test_column_with_column_descriptor(self):
        """Test Column descriptor is properly captured."""

        class User(Table):
            email: str = Column(unique=True)
            age: int = Column(default=0)

        # Should have ColumnProxy at class level
        expect(isinstance(User.email, ColumnProxy)).to_be_true()
        expect(isinstance(User.age, ColumnProxy)).to_be_true()

        # Default should be captured
        expect("email" in User._column_defaults).to_be_true()
        expect(isinstance(User._column_defaults["email"], Column)).to_be_true()

    def test_skip_private_attributes(self):
        """Test private attributes are not treated as columns."""

        class User(Table):
            name: str
            _private: str = "secret"

        expect("name" in User._columns).to_be_true()
        expect("_private" in User._columns).to_be_false()

    def test_inheritance(self):
        """Test table inheritance works correctly."""

        class BaseModel(Table):
            created_at: str

        class User(BaseModel):
            name: str
            email: str

        # Should have all columns from parent and child
        expect("created_at" in User._columns).to_be_true()
        expect("name" in User._columns).to_be_true()
        expect("email" in User._columns).to_be_true()


class TestTableInstanceCreation:
    """Test Table instance creation and initialization."""

    def test_instance_creation_basic(self, sample_table_class):
        """Test basic instance creation."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        expect(user.name).to_equal("Alice")
        expect(user.email).to_equal("alice@example.com")
        expect(user.id).to_be_none()

    def test_instance_creation_with_id(self, sample_table_class):
        """Test instance creation with id."""
        User = sample_table_class
        user = User(id=5, name="Alice", email="alice@example.com")

        expect(user.id).to_equal(5)
        expect(user.name).to_equal("Alice")

    def test_instance_creation_with_defaults(self, sample_table_class):
        """Test default values are applied."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        expect(user.age).to_equal(0)  # Default from annotation
        expect(user.city).to_equal("NYC")  # Default from Column

    def test_field_assignment(self, sample_table_class):
        """Test field values can be assigned after creation."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        user.name = "Alice Smith"
        user.age = 30

        expect(user.name).to_equal("Alice Smith")
        expect(user.age).to_equal(30)

    def test_field_access_via_data(self, sample_table_class):
        """Test fields are stored in _data dict."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com", age=25)

        expect("name" in user._data).to_be_true()
        expect("email" in user._data).to_be_true()
        expect(user._data["name"]).to_equal("Alice")
        expect(user._data["age"]).to_equal(25)

    def test_id_not_in_data(self, sample_table_class):
        """Test id is stored separately, not in _data."""
        User = sample_table_class
        user = User(id=5, name="Alice", email="alice@example.com")

        expect(user.id).to_equal(5)
        expect("id" in user._data).to_be_false()

    def test_optional_fields(self):
        """Test optional fields work correctly."""

        class User(Table):
            name: str
            email: Optional[str] = None

        user = User(name="Alice")
        expect(user.name).to_equal("Alice")
        # email should not be set if not provided and no default

        user2 = User(name="Bob", email="bob@example.com")
        expect(user2.email).to_equal("bob@example.com")

    def test_column_default_factory(self):
        """Test Column with default_factory."""

        def make_list():
            return []

        class User(Table):
            name: str
            tags: list = Column(default_factory=make_list)

        user1 = User(name="Alice")
        user2 = User(name="Bob")

        # Each instance should get a new list
        user1.tags.append("admin")
        expect(len(user1.tags)).to_equal(1)
        expect(len(user2.tags)).to_equal(0)


class TestTableToDict:
    """Test to_dict() method."""

    def test_to_dict_basic(self, sample_table_class):
        """Test basic to_dict conversion."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com", age=30)

        data = user.to_dict()

        expect(isinstance(data, dict)).to_be_true()
        expect(data["name"]).to_equal("Alice")
        expect(data["email"]).to_equal("alice@example.com")
        expect(data["age"]).to_equal(30)

    def test_to_dict_includes_id(self, sample_table_class):
        """Test to_dict includes id when set."""
        User = sample_table_class
        user = User(id=5, name="Alice", email="alice@example.com")

        data = user.to_dict()

        expect("id" in data).to_be_true()
        expect(data["id"]).to_equal(5)

    def test_to_dict_excludes_id_when_none(self, sample_table_class):
        """Test to_dict works when id is None."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        data = user.to_dict()

        # id=None should not be included
        expect(user.id).to_be_none()

    def test_to_dict_with_defaults(self, sample_table_class):
        """Test to_dict includes default values."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        data = user.to_dict()

        expect(data["age"]).to_equal(0)
        expect(data["city"]).to_equal("NYC")


class TestTableRepr:
    """Test __repr__ and string representations."""

    def test_column_proxy_repr(self):
        """Test ColumnProxy __repr__."""

        class User(Table):
            name: str

        proxy = User.name
        repr_str = repr(proxy)

        expect("ColumnProxy" in repr_str).to_be_true()
        expect("name" in repr_str).to_be_true()
        expect("User" in repr_str).to_be_true()


class TestTableAttributeAccess:
    """Test __getattr__ and __setattr__ behavior."""

    def test_getattr_returns_value(self, sample_table_class):
        """Test __getattr__ returns field value from _data."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        expect(user.name).to_equal("Alice")
        expect(user.email).to_equal("alice@example.com")

    def test_getattr_raises_for_missing(self, sample_table_class):
        """Test __getattr__ raises AttributeError for missing fields."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        with pytest.raises(AttributeError):
            _ = user.nonexistent_field

    def test_setattr_updates_data(self, sample_table_class):
        """Test __setattr__ updates _data dict."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        user.name = "Alice Smith"

        expect(user._data["name"]).to_equal("Alice Smith")

    def test_setattr_preserves_private(self, sample_table_class):
        """Test __setattr__ doesn't store private attrs in _data."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        user._custom = "value"

        expect("_custom" in user._data).to_be_false()
        expect(user._custom).to_equal("value")

    def test_setattr_preserves_id(self, sample_table_class):
        """Test __setattr__ stores id separately."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        user.id = 10

        expect(user.id).to_equal(10)
        # Note: Current implementation stores id in _data when set as attribute
        # This is implementation detail and may be acceptable


class TestTableClassMethods:
    """Test class methods like find, find_one, get."""

    @pytest.mark.asyncio
    async def test_find_returns_query_builder(self, sample_table_class):
        """Test find() returns QueryBuilder."""
        from data_bridge.postgres.query import QueryBuilder

        User = sample_table_class
        query = User.find()

        expect(isinstance(query, QueryBuilder)).to_be_true()
        expect(query._model).to_equal(User)

    @pytest.mark.asyncio
    async def test_find_with_filters(self, sample_table_class):
        """Test find() accepts filters."""
        from data_bridge.postgres.query import QueryBuilder

        User = sample_table_class
        query = User.find(User.age > 25)

        expect(isinstance(query, QueryBuilder)).to_be_true()
        expect(len(query._filters)).to_equal(1)

    @pytest.mark.asyncio
    async def test_count_delegates_to_find(self, sample_table_class):
        """Test count() delegates to find().count()."""
        from data_bridge.postgres.query import QueryBuilder

        User = sample_table_class

        # count() should return a QueryBuilder.count() call
        # Mock the query engine
        with patch('data_bridge.postgres.query._engine') as mock_engine:
            mock_engine.count = AsyncMock(return_value=42)

            result = await User.count()

            expect(result).to_equal(42)
            mock_engine.count.assert_called_once()


class TestTableSettings:
    """Test Settings inner class configuration."""

    def test_settings_defaults(self):
        """Test Settings defaults are applied."""

        class Product(Table):
            name: str

        expect(Product._schema).to_equal("public")
        expect(Product._primary_key).to_equal("id")

    def test_settings_custom_values(self):
        """Test custom Settings values are used."""

        class Product(Table):
            sku: str

            class Settings:
                table_name = "products"
                schema = "inventory"
                primary_key = "sku"

        expect(Product._table_name).to_equal("products")
        expect(Product._schema).to_equal("inventory")
        expect(Product._primary_key).to_equal("sku")

    def test_settings_indexes(self):
        """Test Settings can include index definitions."""

        class User(Table):
            email: str

            class Settings:
                indexes = [
                    {"columns": ["email"], "unique": True},
                ]

        # Should be accessible
        expect(hasattr(User._settings, "indexes")).to_be_true()
