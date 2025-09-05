"""Tests for base model module."""

import pytest

from data_bridge.base.fields import Field
from data_bridge.base.model import BaseModel


class MockTestModel(BaseModel):
    """Test model for testing BaseModel functionality."""

    name = Field(required=True)
    age = Field(default=25, required=False)
    tags = Field(default=None, required=False)


class TestBaseModel:
    """Test BaseModel class."""

    def test_model_creation_with_kwargs(self) -> None:
        """Test model instance creation with kwargs."""
        instance = MockTestModel(name="John", age=30, tags=[])
        assert instance.name == "John"
        assert instance.age == 30
        assert instance.tags == []

    def test_model_creation_with_defaults(self) -> None:
        """Test model instance creation with default values."""
        instance = MockTestModel(name="John")
        assert instance.name == "John"
        assert instance.age == 25  # default value
        assert instance.tags is None  # default value

    def test_model_creation_missing_required_field(self) -> None:
        """Test that missing required field raises error."""
        with pytest.raises(ValueError, match="Required field 'name' not provided"):
            MockTestModel(age=30)

    def test_model_to_dict(self) -> None:
        """Test model to_dict method."""
        instance = MockTestModel(name="John", age=30, tags=[])
        data = instance.to_dict()

        expected = {"name": "John", "age": 30, "tags": []}
        assert data == expected

    def test_model_to_dict_with_db_field(self) -> None:
        """Test model to_dict with custom db_field names."""

        class MockTestModelWithDbField(BaseModel):
            name = Field(db_field="full_name", required=True)
            age = Field(required=True)

        instance = MockTestModelWithDbField(name="John", age=30)
        data = instance.to_dict()

        expected = {"full_name": "John", "age": 30}
        assert data == expected

    def test_model_to_dict_excludes_none_values(self) -> None:
        """Test that to_dict excludes None values."""

        class MockTestModelOptional(BaseModel):
            name = Field(required=True)
            description = Field(required=False)

        instance = MockTestModelOptional(name="John")
        data = instance.to_dict()

        # description should not be in dict since it's None
        assert data == {"name": "John"}

    def test_model_from_dict(self) -> None:
        """Test model from_dict class method."""
        data = {"name": "John", "age": 30, "tags": ["python", "coding"]}
        instance = MockTestModel.from_dict(data)

        assert instance.name == "John"
        assert instance.age == 30
        assert instance.tags == ["python", "coding"]

    def test_model_from_dict_with_db_field(self) -> None:
        """Test model from_dict with custom db_field names."""

        class MockTestModelWithDbField(BaseModel):
            name = Field(db_field="full_name", required=True)
            age = Field(required=True)

        data = {"full_name": "John", "age": 30}
        instance = MockTestModelWithDbField.from_dict(data)

        assert instance.name == "John"
        assert instance.age == 30

    def test_model_from_dict_partial_data(self) -> None:
        """Test model from_dict with partial data."""
        data = {"name": "John"}  # missing age, tags
        instance = MockTestModel.from_dict(data)

        assert instance.name == "John"
        # These should be set to defaults during initialization
        assert instance.age == 25
        assert instance.tags is None

    def test_model_repr(self) -> None:
        """Test model __repr__ method."""
        instance = MockTestModel(name="John", age=30, tags=[])
        repr_str = repr(instance)

        assert "MockTestModel" in repr_str
        assert "name='John'" in repr_str
        assert "age=30" in repr_str
        assert "tags=[]" in repr_str

    def test_model_repr_with_none_values(self) -> None:
        """Test model __repr__ excludes None values."""

        class MockTestModelOptional(BaseModel):
            name = Field(required=True)
            description = Field(required=False)

        instance = MockTestModelOptional(name="John")
        repr_str = repr(instance)

        assert "MockTestModelOptional" in repr_str
        assert "name='John'" in repr_str
        assert "description" not in repr_str

    def test_model_equality(self) -> None:
        """Test model equality comparison."""
        instance1 = MockTestModel(name="John", age=30, tags=[])
        instance2 = MockTestModel(name="John", age=30, tags=[])
        instance3 = MockTestModel(name="Jane", age=30, tags=[])

        assert instance1 == instance2
        assert instance1 != instance3

    def test_model_equality_different_class(self) -> None:
        """Test model equality with different class."""

        class OtherModel(BaseModel):
            name = Field(required=True)
            age = Field(required=True)

        instance1 = MockTestModel(name="John", age=30, tags=[])
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
        MockTestModel.set_backend(backend)

        assert MockTestModel._backend is backend


class TestBaseModelFieldDefaults:
    """Test BaseModel field default behavior."""
    
    def test_model_with_default_factory(self):
        """Test model with default_factory field."""
        def make_list():
            return ["default", "items"]
        
        class ModelWithFactory(BaseModel):
            name = Field(required=True)
            items = Field(default_factory=make_list, required=False)
        
        instance = ModelWithFactory(name="test")
        assert instance.name == "test"
        assert instance.items == ["default", "items"]
        
        # Factory should be called for each instance
        instance2 = ModelWithFactory(name="test2")
        assert instance2.items == ["default", "items"]
        # Should be different objects
        assert instance.items is not instance2.items
    
    def test_model_field_assignment_after_creation(self):
        """Test assigning field values after creation."""
        instance = MockTestModel(name="John")
        
        # Should be able to assign new values
        instance.age = 35
        instance.tags = ["new", "tags"]
        
        assert instance.age == 35
        assert instance.tags == ["new", "tags"]


class TestBaseModelEdgeCases:
    """Test edge cases for BaseModel."""
    
    def test_empty_model(self):
        """Test model with no fields."""
        class EmptyModel(BaseModel):
            pass
        
        instance = EmptyModel()
        
        # Should work without errors
        data = instance.to_dict()
        assert data == {}
        
        repr_str = repr(instance)
        assert repr_str == "EmptyModel()"
        
        # Test equality
        other = EmptyModel()
        assert instance == other
    
    def test_model_with_only_optional_fields(self):
        """Test model where all fields are optional."""
        class OptionalModel(BaseModel):
            name = Field(default="Anonymous", required=False)
            age = Field(default=None, required=False)
        
        # Should work with no arguments
        instance = OptionalModel()
        assert instance.name == "Anonymous"
        assert instance.age is None
        
        # Should work with some arguments
        instance2 = OptionalModel(name="John")
        assert instance2.name == "John"
        assert instance2.age is None
    
    def test_model_equality_with_none_fields(self):
        """Test equality when some fields are None."""
        class OptionalModel(BaseModel):
            name = Field(required=True)
            description = Field(required=False)
        
        instance1 = OptionalModel(name="John")  # description is None
        instance2 = OptionalModel(name="John")  # description is None
        instance3 = OptionalModel(name="John", description="A person")
        
        assert instance1 == instance2
        assert instance1 != instance3
    
    def test_model_from_dict_ignores_unknown_fields(self):
        """Test from_dict ignores fields not in model."""
        data = {
            "name": "John",
            "age": 30,
            "unknown_field": "should be ignored",
            "another_unknown": 123
        }
        
        instance = MockTestModel.from_dict(data)
        assert instance.name == "John"
        assert instance.age == 30
        
        # Unknown fields should not be set
        assert not hasattr(instance, "unknown_field")
        assert not hasattr(instance, "another_unknown")
    
    def test_model_to_dict_all_none(self):
        """Test to_dict when all optional fields are None."""
        class OptionalModel(BaseModel):
            name = Field(required=True)
            description = Field(required=False)
            tags = Field(required=False)
        
        instance = OptionalModel(name="John")
        data = instance.to_dict()
        
        # Only name should be in dict
        assert data == {"name": "John"}
    
    def test_model_inheritance_complex(self):
        """Test complex model inheritance scenario."""
        class BaseUser(BaseModel):
            id = Field(required=True)
            name = Field(required=True)
        
        class ExtendedUser(BaseUser):
            email = Field(required=False)
            active = Field(default=True, required=False)
        
        class AdminUser(ExtendedUser):
            permissions = Field(default=[], required=False)
            last_login = Field(required=False)
        
        # Create admin user
        admin = AdminUser(
            id="admin1", 
            name="Admin User", 
            email="admin@example.com",
            permissions=["read", "write", "admin"]
        )
        
        assert admin.id == "admin1"
        assert admin.name == "Admin User"
        assert admin.email == "admin@example.com"
        assert admin.active is True  # Default from ExtendedUser
        assert admin.permissions == ["read", "write", "admin"]
        assert admin.last_login is None
        
        # Test to_dict includes all non-None fields
        data = admin.to_dict()
        expected = {
            "id": "admin1",
            "name": "Admin User",
            "email": "admin@example.com", 
            "active": True,
            "permissions": ["read", "write", "admin"]
        }
        assert data == expected
    
    def test_model_field_override_in_inheritance(self):
        """Test overriding parent field in child class."""
        class ParentModel(BaseModel):
            status = Field(default="pending", required=False)
            name = Field(required=True)
        
        class ChildModel(ParentModel):
            # Override status with different default
            status = Field(default="active", required=False)
            extra = Field(required=False)
        
        instance = ChildModel(name="test")
        assert instance.name == "test"
        assert instance.status == "active"  # Child default, not parent
        assert instance.extra is None
    
    def test_model_metaclass_interaction(self):
        """Test model works correctly with metaclass."""
        # This tests the metaclass sets up _fields correctly
        assert hasattr(MockTestModel, '_fields')
        assert 'name' in MockTestModel._fields
        assert 'age' in MockTestModel._fields
        assert 'tags' in MockTestModel._fields
        
        # Test field objects are properly set up
        name_field = MockTestModel._fields['name']
        assert name_field.required is True
        
        age_field = MockTestModel._fields['age']
        assert age_field.required is False
        assert age_field.default == 25
