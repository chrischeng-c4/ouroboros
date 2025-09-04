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
