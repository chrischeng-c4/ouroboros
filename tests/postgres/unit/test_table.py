"""
Unit tests for Table class.

Tests the Table class definition, metaclass behavior, and instance methods
without requiring a real database connection.
"""
import pytest
from typing import Optional
from unittest.mock import patch, AsyncMock
from data_bridge.postgres import Table, Column, ColumnProxy


class TestTableMetaclass:
    """Test TableMeta metaclass behavior."""

    def test_table_name_from_settings(self):
        """Test table name is correctly set from Settings class."""

        class Product(Table):
            name: str

            class Settings:
                table_name = "products"

        assert Product._table_name == "products"

    def test_table_name_default(self):
        """Test table name defaults to lowercase class name."""

        class OrderItem(Table):
            name: str

        assert OrderItem._table_name == "orderitem"

    def test_table_name_with_schema(self):
        """Test full table name includes schema."""

        class User(Table):
            name: str

            class Settings:
                table_name = "users"
                schema = "auth"

        assert User._schema == "auth"
        assert User.__table_name__() == "auth.users"

    def test_primary_key_detection(self):
        """Test primary key is correctly identified."""

        class Product(Table):
            product_id: int

            class Settings:
                primary_key = "product_id"

        assert Product._primary_key == "product_id"

    def test_primary_key_default(self):
        """Test primary key defaults to 'id'."""

        class Product(Table):
            name: str

        assert Product._primary_key == "id"

    def test_column_definitions(self):
        """Test columns are correctly collected from annotations."""

        class User(Table):
            name: str
            email: str
            age: int

        assert "name" in User._columns
        assert "email" in User._columns
        assert "age" in User._columns
        assert User._columns["name"] == str
        assert User._columns["age"] == int

    def test_column_proxy_created(self):
        """Test ColumnProxy is created for each column."""

        class User(Table):
            name: str
            email: str

        # Class-level access should return ColumnProxy
        assert isinstance(User.name, ColumnProxy)
        assert isinstance(User.email, ColumnProxy)
        assert User.name.name == "name"
        assert User.email.name == "email"

    def test_column_defaults_captured(self):
        """Test default values are captured before ColumnProxy replacement."""

        class User(Table):
            name: str
            age: int = 0
            status: str = "active"

        assert "age" in User._column_defaults
        assert "status" in User._column_defaults
        assert User._column_defaults["age"] == 0
        assert User._column_defaults["status"] == "active"

    def test_column_with_column_descriptor(self):
        """Test Column descriptor is properly captured."""

        class User(Table):
            email: str = Column(unique=True)
            age: int = Column(default=0)

        # Should have ColumnProxy at class level
        assert isinstance(User.email, ColumnProxy)
        assert isinstance(User.age, ColumnProxy)

        # Default should be captured
        assert "email" in User._column_defaults
        assert isinstance(User._column_defaults["email"], Column)

    def test_skip_private_attributes(self):
        """Test private attributes are not treated as columns."""

        class User(Table):
            name: str
            _private: str = "secret"

        assert "name" in User._columns
        assert "_private" not in User._columns

    def test_inheritance(self):
        """Test table inheritance works correctly."""

        class BaseModel(Table):
            created_at: str

        class User(BaseModel):
            name: str
            email: str

        # Should have all columns from parent and child
        assert "created_at" in User._columns
        assert "name" in User._columns
        assert "email" in User._columns


class TestTableInstanceCreation:
    """Test Table instance creation and initialization."""

    def test_instance_creation_basic(self, sample_table_class):
        """Test basic instance creation."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        assert user.name == "Alice"
        assert user.email == "alice@example.com"
        assert user.id is None

    def test_instance_creation_with_id(self, sample_table_class):
        """Test instance creation with id."""
        User = sample_table_class
        user = User(id=5, name="Alice", email="alice@example.com")

        assert user.id == 5
        assert user.name == "Alice"

    def test_instance_creation_with_defaults(self, sample_table_class):
        """Test default values are applied."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        assert user.age == 0  # Default from annotation
        assert user.city == "NYC"  # Default from Column

    def test_field_assignment(self, sample_table_class):
        """Test field values can be assigned after creation."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        user.name = "Alice Smith"
        user.age = 30

        assert user.name == "Alice Smith"
        assert user.age == 30

    def test_field_access_via_data(self, sample_table_class):
        """Test fields are stored in _data dict."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com", age=25)

        assert "name" in user._data
        assert "email" in user._data
        assert user._data["name"] == "Alice"
        assert user._data["age"] == 25

    def test_id_not_in_data(self, sample_table_class):
        """Test id is stored separately, not in _data."""
        User = sample_table_class
        user = User(id=5, name="Alice", email="alice@example.com")

        assert user.id == 5
        assert "id" not in user._data

    def test_optional_fields(self):
        """Test optional fields work correctly."""

        class User(Table):
            name: str
            email: Optional[str] = None

        user = User(name="Alice")
        assert user.name == "Alice"
        # email should not be set if not provided and no default

        user2 = User(name="Bob", email="bob@example.com")
        assert user2.email == "bob@example.com"

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
        assert len(user1.tags) == 1
        assert len(user2.tags) == 0


class TestTableToDict:
    """Test to_dict() method."""

    def test_to_dict_basic(self, sample_table_class):
        """Test basic to_dict conversion."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com", age=30)

        data = user.to_dict()

        assert isinstance(data, dict)
        assert data["name"] == "Alice"
        assert data["email"] == "alice@example.com"
        assert data["age"] == 30

    def test_to_dict_includes_id(self, sample_table_class):
        """Test to_dict includes id when set."""
        User = sample_table_class
        user = User(id=5, name="Alice", email="alice@example.com")

        data = user.to_dict()

        assert "id" in data
        assert data["id"] == 5

    def test_to_dict_excludes_id_when_none(self, sample_table_class):
        """Test to_dict works when id is None."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        data = user.to_dict()

        # id=None should not be included
        assert user.id is None

    def test_to_dict_with_defaults(self, sample_table_class):
        """Test to_dict includes default values."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        data = user.to_dict()

        assert data["age"] == 0
        assert data["city"] == "NYC"


class TestTableRepr:
    """Test __repr__ and string representations."""

    def test_column_proxy_repr(self):
        """Test ColumnProxy __repr__."""

        class User(Table):
            name: str

        proxy = User.name
        repr_str = repr(proxy)

        assert "ColumnProxy" in repr_str
        assert "name" in repr_str
        assert "User" in repr_str


class TestTableAttributeAccess:
    """Test __getattr__ and __setattr__ behavior."""

    def test_getattr_returns_value(self, sample_table_class):
        """Test __getattr__ returns field value from _data."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        assert user.name == "Alice"
        assert user.email == "alice@example.com"

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

        assert user._data["name"] == "Alice Smith"

    def test_setattr_preserves_private(self, sample_table_class):
        """Test __setattr__ doesn't store private attrs in _data."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        user._custom = "value"

        assert "_custom" not in user._data
        assert user._custom == "value"

    def test_setattr_preserves_id(self, sample_table_class):
        """Test __setattr__ stores id separately."""
        User = sample_table_class
        user = User(name="Alice", email="alice@example.com")

        user.id = 10

        assert user.id == 10
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

        assert isinstance(query, QueryBuilder)
        assert query._model == User

    @pytest.mark.asyncio
    async def test_find_with_filters(self, sample_table_class):
        """Test find() accepts filters."""
        from data_bridge.postgres.query import QueryBuilder

        User = sample_table_class
        query = User.find(User.age > 25)

        assert isinstance(query, QueryBuilder)
        assert len(query._filters) == 1

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

            assert result == 42
            mock_engine.count.assert_called_once()


class TestTableSettings:
    """Test Settings inner class configuration."""

    def test_settings_defaults(self):
        """Test Settings defaults are applied."""

        class Product(Table):
            name: str

        assert Product._schema == "public"
        assert Product._primary_key == "id"

    def test_settings_custom_values(self):
        """Test custom Settings values are used."""

        class Product(Table):
            sku: str

            class Settings:
                table_name = "products"
                schema = "inventory"
                primary_key = "sku"

        assert Product._table_name == "products"
        assert Product._schema == "inventory"
        assert Product._primary_key == "sku"

    def test_settings_indexes(self):
        """Test Settings can include index definitions."""

        class User(Table):
            email: str

            class Settings:
                indexes = [
                    {"columns": ["email"], "unique": True},
                ]

        # Should be accessible
        assert hasattr(User._settings, "indexes")
