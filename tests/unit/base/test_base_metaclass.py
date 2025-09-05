"""Tests for base metaclass module."""


import pytest

from data_bridge.base.fields import Field
from data_bridge.base.metaclass import ModelMetaclass


class TestModelMetaclass:
    """Test ModelMetaclass."""

    def test_metaclass_basic_functionality(self) -> None:
        """Test basic metaclass functionality."""

        class TestModel(metaclass=ModelMetaclass):
            name = Field(required=True)
            age = Field(default=0)

        # Check that fields are collected
        assert hasattr(TestModel, "_fields")
        assert "name" in TestModel._fields
        assert "age" in TestModel._fields
        assert isinstance(TestModel._fields["name"], Field)
        assert isinstance(TestModel._fields["age"], Field)

        # Check default collection name
        assert hasattr(TestModel, "_collection")
        assert TestModel._collection == "testmodels"

        # Check database
        assert hasattr(TestModel, "_database")
        assert TestModel._database is None

    def test_metaclass_with_primary_key(self) -> None:
        """Test metaclass with primary key field."""

        class TestModel(metaclass=ModelMetaclass):
            id = Field(primary_key=True, required=True)
            name = Field()

        assert hasattr(TestModel, "_pk_field")
        assert TestModel._pk_field is TestModel._fields["id"]

    def test_metaclass_multiple_primary_keys_error(self) -> None:
        """Test that multiple primary keys raise error."""

        with pytest.raises(ValueError, match="multiple primary key fields"):
            class TestModel(metaclass=ModelMetaclass):
                id1 = Field(primary_key=True)
                id2 = Field(primary_key=True)

    def test_metaclass_inheritance(self) -> None:
        """Test field inheritance through metaclass."""

        class BaseModel(metaclass=ModelMetaclass):
            created_at = Field(default="now")

        class TestModel(BaseModel):
            name = Field(required=True)

        # Both fields should be present
        assert "created_at" in TestModel._fields
        assert "name" in TestModel._fields
        assert len(TestModel._fields) == 2

        # Base class should only have its field
        assert "created_at" in BaseModel._fields
        assert "name" not in BaseModel._fields
        assert len(BaseModel._fields) == 1

    def test_metaclass_custom_collection_name(self) -> None:
        """Test custom collection name."""

        class TestModel(metaclass=ModelMetaclass, collection="custom_collection"):
            name = Field()

        assert TestModel._collection == "custom_collection"

    def test_metaclass_custom_database_name(self) -> None:
        """Test custom database name."""

        class TestModel(metaclass=ModelMetaclass, database="custom_db"):
            name = Field()

        assert TestModel._database == "custom_db"

    def test_metaclass_skips_internal_classes(self) -> None:
        """Test that metaclass skips internal classes for type hints."""

        # This should not raise an error even without proper type hints
        class _InternalModel(metaclass=ModelMetaclass):
            name = Field()

        class Model(metaclass=ModelMetaclass):
            name = Field()

        # Both should work
        assert hasattr(_InternalModel, "_fields")
        assert hasattr(Model, "_fields")

    def test_metaclass_with_type_hints(self) -> None:
        """Test metaclass with type hints for fields."""

        class TypedModel(metaclass=ModelMetaclass):
            name: str = Field(required=True)
            age: int = Field(default=0)

        # Fields should have type information set
        name_field = TypedModel._fields["name"]
        age_field = TypedModel._fields["age"]
        
        # Type information should be available
        assert hasattr(name_field, 'type')
        assert hasattr(age_field, 'type')
        # The types should be set (though might be str representation)
        assert name_field.type is not None
        assert age_field.type is not None

    def test_metaclass_no_primary_key(self) -> None:
        """Test metaclass with no primary key fields."""

        class NoPKModel(metaclass=ModelMetaclass):
            name = Field(required=True)
            age = Field(default=0)

        assert hasattr(NoPKModel, "_pk_field")
        assert NoPKModel._pk_field is None

    def test_metaclass_field_override_in_inheritance(self) -> None:
        """Test that child classes can override parent field definitions."""

        class BaseModel(metaclass=ModelMetaclass):
            status = Field(default="pending")
            created_at = Field(default="now")

        class ChildModel(BaseModel):
            # Override status field
            status = Field(default="active", required=True)
            name = Field(required=True)

        # Child should have overridden status field
        assert "status" in ChildModel._fields
        assert "created_at" in ChildModel._fields
        assert "name" in ChildModel._fields
        assert len(ChildModel._fields) == 3

        # Check that the status field is the overridden one
        child_status_field = ChildModel._fields["status"]
        base_status_field = BaseModel._fields["status"]
        
        # They should be different objects (overridden)
        assert child_status_field is not base_status_field
        assert child_status_field.default == "active"
        assert child_status_field.required is True

    def test_metaclass_deep_inheritance_chain(self) -> None:
        """Test metaclass with deep inheritance chain."""

        class GrandParentModel(metaclass=ModelMetaclass):
            created_at = Field(default="now")

        class ParentModel(GrandParentModel):
            updated_at = Field(default="updated")

        class ChildModel(ParentModel):
            name = Field(required=True)

        # All fields should be inherited
        assert "created_at" in ChildModel._fields
        assert "updated_at" in ChildModel._fields  
        assert "name" in ChildModel._fields
        assert len(ChildModel._fields) == 3

        # Each level should have appropriate fields
        assert len(GrandParentModel._fields) == 1
        assert len(ParentModel._fields) == 2
        assert len(ChildModel._fields) == 3

    def test_metaclass_with_both_collection_and_database(self) -> None:
        """Test metaclass with both collection and database specified."""

        class FullySpecifiedModel(metaclass=ModelMetaclass, 
                                  collection="custom_collection",
                                  database="custom_database"):
            name = Field(required=True)

        assert FullySpecifiedModel._collection == "custom_collection"
        assert FullySpecifiedModel._database == "custom_database"

    def test_metaclass_complex_field_types(self) -> None:
        """Test metaclass with complex field types and attributes."""

        class ComplexModel(metaclass=ModelMetaclass):
            id = Field(primary_key=True, required=True, unique=True, index=True)
            name = Field(required=True, db_field="full_name")
            tags = Field(default=[], required=False)
            active = Field(default=True, required=False)

        # Check primary key field
        assert ComplexModel._pk_field is ComplexModel._fields["id"]
        assert ComplexModel._fields["id"].primary_key is True
        assert ComplexModel._fields["id"].unique is True
        assert ComplexModel._fields["id"].index is True

        # Check db_field mapping
        assert ComplexModel._fields["name"].db_field == "full_name"

        # Check defaults
        assert ComplexModel._fields["tags"].default == []
        assert ComplexModel._fields["active"].default is True
