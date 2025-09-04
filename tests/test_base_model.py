"""Tests for base model module."""

import pytest

from data_bridge.base.fields import Field
from data_bridge.base.model import BaseModel


class TestModel(BaseModel):
    """Test model for testing BaseModel functionality."""

    name = Field(required=True)
    age = Field(default=25, required=False)
    tags = Field(default=None, required=False)


class TestBaseModel:
    """Test BaseModel class."""

    def test_model_creation_with_kwargs(self) -> None:
        """Test model instance creation with kwargs."""
        instance = TestModel(name="John", age=30, tags=[])
        assert instance.name == "John"
        assert instance.age == 30
        assert instance.tags == []

    def test_model_creation_with_defaults(self) -> None:
        """Test model instance creation with default values."""
        instance = TestModel(name="John")
        assert instance.name == "John"
        assert instance.age == 25  # default value
        assert instance.tags is None  # default value

    def test_model_creation_missing_required_field(self) -> None:
        """Test that missing required field raises error."""
        with pytest.raises(ValueError, match="Required field 'name' not provided"):
            TestModel(age=30)

    def test_model_to_dict(self) -> None:
        """Test model to_dict method."""
        instance = TestModel(name="John", age=30, tags=[])
        data = instance.to_dict()

        expected = {"name": "John", "age": 30, "tags": []}
        assert data == expected

    def test_model_to_dict_with_db_field(self) -> None:
        """Test model to_dict with custom db_field names."""

        class TestModelWithDbField(BaseModel):
            name = Field(db_field="full_name", required=True)
            age = Field(required=True)

        instance = TestModelWithDbField(name="John", age=30)
        data = instance.to_dict()

        expected = {"full_name": "John", "age": 30}
        assert data == expected

    def test_model_to_dict_excludes_none_values(self) -> None:
        """Test that to_dict excludes None values."""

        class TestModelOptional(BaseModel):
            name = Field(required=True)
            description = Field(required=False)

        instance = TestModelOptional(name="John")
        data = instance.to_dict()

        # description should not be in dict since it's None
        assert data == {"name": "John"}

    def test_model_from_dict(self) -> None:
        """Test model from_dict class method."""
        data = {"name": "John", "age": 30, "tags": ["python", "coding"]}
        instance = TestModel.from_dict(data)

        assert instance.name == "John"
        assert instance.age == 30
        assert instance.tags == ["python", "coding"]

    def test_model_from_dict_with_db_field(self) -> None:
        """Test model from_dict with custom db_field names."""

        class TestModelWithDbField(BaseModel):
            name = Field(db_field="full_name", required=True)
            age = Field(required=True)

        data = {"full_name": "John", "age": 30}
        instance = TestModelWithDbField.from_dict(data)

        assert instance.name == "John"
        assert instance.age == 30

    def test_model_from_dict_partial_data(self) -> None:
        """Test model from_dict with partial data."""
        data = {"name": "John"}  # missing age, tags
        instance = TestModel.from_dict(data)

        assert instance.name == "John"
        # These should be set to defaults during initialization
        assert instance.age == 25
        assert instance.tags is None

    def test_model_repr(self) -> None:
        """Test model __repr__ method."""
        instance = TestModel(name="John", age=30, tags=[])
        repr_str = repr(instance)

        assert "TestModel" in repr_str
        assert "name='John'" in repr_str
        assert "age=30" in repr_str
        assert "tags=[]" in repr_str

    def test_model_repr_with_none_values(self) -> None:
        """Test model __repr__ excludes None values."""

        class TestModelOptional(BaseModel):
            name = Field(required=True)
            description = Field(required=False)

        instance = TestModelOptional(name="John")
        repr_str = repr(instance)

        assert "TestModelOptional" in repr_str
        assert "name='John'" in repr_str
        assert "description" not in repr_str

    def test_model_equality(self) -> None:
        """Test model equality comparison."""
        instance1 = TestModel(name="John", age=30, tags=[])
        instance2 = TestModel(name="John", age=30, tags=[])
        instance3 = TestModel(name="Jane", age=30, tags=[])

        assert instance1 == instance2
        assert instance1 != instance3

    def test_model_equality_different_class(self) -> None:
        """Test model equality with different class."""

        class OtherModel(BaseModel):
            name = Field(required=True)
            age = Field(required=True)

        instance1 = TestModel(name="John", age=30, tags=[])
        instance2 = OtherModel(name="John", age=30)

        assert instance1 != instance2

    def test_set_backend(self) -> None:
        """Test set_backend class method."""
        from data_bridge.base.backends.sync import SyncBackend

        class MockBackend(SyncBackend):
            def save(self, instance) -> None:
                pass

            def delete(self, instance) -> None:
                pass

            def execute_query(self, query):
                return []

            def count_query(self, query) -> int:
                return 0

            def delete_query(self, query) -> int:
                return 0

            def update_query(self, query, updates) -> int:
                return 0

        backend = MockBackend()
        TestModel.set_backend(backend)

        assert TestModel._backend is backend
