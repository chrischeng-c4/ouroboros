"""Tests for base fields module."""

import pytest
from typing import Any

from data_bridge.base.fields import (
    CompoundExpression, Field, QueryExpression, UpdateExpression,
    IntField, FloatField, StringField, BoolField, 
    ListField, DictField, NestedFieldProxy
)


class TestField:
    """Test Field class."""

    def test_field_creation(self) -> None:
        """Test basic field creation."""
        field = Field(default="test", required=False)
        assert field.default == "test"
        assert not field.required
        assert not field.primary_key
        assert field.db_field is None

    def test_field_required(self) -> None:
        """Test required field."""
        field = Field(required=True)
        assert field.required
        assert field.default is None

    def test_field_primary_key(self) -> None:
        """Test primary key field."""
        field = Field(primary_key=True, required=True)
        assert field.primary_key
        assert field.required

    def test_field_db_field(self) -> None:
        """Test field with custom db_field name."""
        field = Field(db_field="custom_name")
        assert field.db_field == "custom_name"

    def test_field_equality_operator(self) -> None:
        """Test field equality operator."""
        field = Field()
        field.name = "test_field"  # Simulate what metaclass would do
        expr = field == "test"
        assert isinstance(expr, QueryExpression)
        assert expr.field == "test_field"
        assert expr.operator == "eq"
        assert expr.value == "test"

    def test_field_comparison_operators(self) -> None:
        """Test field comparison operators."""
        field = Field()
        field.name = "age"  # Simulate what metaclass would do

        # Greater than
        expr = field > 5
        assert isinstance(expr, QueryExpression)
        assert expr.field == "age"
        assert expr.operator == "gt"
        assert expr.value == 5

        # Greater than or equal
        expr = field >= 5
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "gte"
        assert expr.value == 5

        # Less than
        expr = field < 5
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "lt"
        assert expr.value == 5

        # Less than or equal
        expr = field <= 5
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "lte"
        assert expr.value == 5

        # Not equal
        expr = field != 5
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "ne"
        assert expr.value == 5

    def test_field_in_operator(self) -> None:
        """Test field in operator."""
        field = Field()
        field.name = "status"
        expr = field.in_(["a", "b", "c"])
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "in"
        assert expr.value == ["a", "b", "c"]

    def test_field_not_in_operator(self) -> None:
        """Test field not in operator."""
        field = Field()
        field.name = "status"
        expr = field.not_in(["a", "b", "c"])
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "not_in"
        assert expr.value == ["a", "b", "c"]

    def test_field_string_operators(self) -> None:
        """Test field string operators."""
        field = Field()
        field.name = "name"

        # Contains
        expr = field.contains("john")
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "contains"
        assert expr.value == "john"

        # Starts with
        expr = field.startswith("Mr.")
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "startswith"
        assert expr.value == "Mr."

        # Ends with
        expr = field.endswith(".com")
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "endswith"
        assert expr.value == ".com"

    def test_field_exists_operator(self) -> None:
        """Test field exists operator."""
        field = Field()
        field.name = "optional_field"

        expr = field.exists()
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "exists"
        assert expr.value is True

        expr = field.exists(False)
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "exists"
        assert expr.value is False


class TestQueryExpression:
    """Test QueryExpression class."""

    def test_query_expression_creation(self) -> None:
        """Test QueryExpression creation."""
        expr = QueryExpression(field="name", operator="eq", value="test")
        assert expr.field == "name"
        assert expr.operator == "eq"
        assert expr.value == "test"

    def test_query_expression_and(self) -> None:
        """Test QueryExpression AND operation."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)

        compound = expr1 & expr2
        assert isinstance(compound, CompoundExpression)
        assert compound.operator == "and"
        assert len(compound.expressions) == 2
        assert compound.expressions[0] == expr1
        assert compound.expressions[1] == expr2

    def test_query_expression_or(self) -> None:
        """Test QueryExpression OR operation."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)

        compound = expr1 | expr2
        assert isinstance(compound, CompoundExpression)
        assert compound.operator == "or"
        assert len(compound.expressions) == 2
        assert compound.expressions[0] == expr1
        assert compound.expressions[1] == expr2

    def test_query_expression_not(self) -> None:
        """Test QueryExpression NOT operation."""
        expr = QueryExpression(field="name", operator="eq", value="test")

        compound = ~expr
        assert isinstance(compound, CompoundExpression)
        assert compound.operator == "not"
        assert len(compound.expressions) == 1
        assert compound.expressions[0] == expr


class TestCompoundExpression:
    """Test CompoundExpression class."""

    def test_compound_expression_creation(self) -> None:
        """Test CompoundExpression creation."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)

        compound = CompoundExpression("and", [expr1, expr2])
        assert compound.operator == "and"
        assert len(compound.expressions) == 2

    def test_compound_expression_and(self) -> None:
        """Test CompoundExpression AND with another expression."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        compound1 = CompoundExpression("and", [expr1, expr2])

        expr3 = QueryExpression(field="status", operator="eq", value="active")
        compound2 = compound1 & expr3

        assert isinstance(compound2, CompoundExpression)
        assert compound2.operator == "and"
        assert len(compound2.expressions) == 2
        assert compound2.expressions[0] == compound1
        assert compound2.expressions[1] == expr3

    def test_compound_expression_or(self) -> None:
        """Test CompoundExpression OR with another expression."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        compound1 = CompoundExpression("and", [expr1, expr2])

        expr3 = QueryExpression(field="status", operator="eq", value="inactive")
        compound2 = compound1 | expr3

        assert isinstance(compound2, CompoundExpression)
        assert compound2.operator == "or"
        assert len(compound2.expressions) == 2
        assert compound2.expressions[0] == compound1
        assert compound2.expressions[1] == expr3

    def test_compound_expression_not(self) -> None:
        """Test CompoundExpression NOT operation."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        compound = CompoundExpression("and", [expr1, expr2])

        negated = ~compound
        assert isinstance(negated, CompoundExpression)
        assert negated.operator == "not"
        assert len(negated.expressions) == 1
        assert negated.expressions[0] == compound


class TestUpdateExpression:
    """Test UpdateExpression functionality."""
    
    def test_update_expression_creation(self):
        """Test creating an UpdateExpression."""
        expr = UpdateExpression("name", "set", "John")
        assert expr.field == "name"
        assert expr.operator == "set"
        assert expr.value == "John"
        assert expr.modifiers is None
    
    def test_update_expression_with_modifiers(self):
        """Test creating UpdateExpression with modifiers."""
        modifiers = {"$position": 0, "$slice": 5}
        expr = UpdateExpression("tags", "push", "new_tag", modifiers)
        
        assert expr.field == "tags"
        assert expr.operator == "push"
        assert expr.value == "new_tag"
        assert expr.modifiers == modifiers
    
    def test_update_expression_validation_empty_field(self):
        """Test validation with empty field."""
        with pytest.raises(ValueError, match="Field cannot be empty"):
            UpdateExpression("", "set", "value")
    
    def test_update_expression_validation_empty_operator(self):
        """Test validation with empty operator."""
        with pytest.raises(ValueError, match="Operator cannot be empty"):
            UpdateExpression("field", "", "value")


class TestFieldDescriptorBehavior:
    """Test Field descriptor behavior."""
    
    def test_field_initialization_with_factory(self):
        """Test field with default_factory."""
        def factory():
            return "factory_value"
        
        field = Field[str](default_factory=factory, required=False)
        assert field.default_factory == factory
        assert field.default is None
    
    def test_field_default_and_factory_conflict(self):
        """Test that specifying both default and default_factory raises error."""
        with pytest.raises(ValueError, match="Cannot specify both default and default_factory"):
            Field[str](default="test", default_factory=lambda: "factory")
    
    def test_field_set_name(self):
        """Test __set_name__ method."""
        field = Field[str]()
        
        class TestModel:
            name = field
        
        assert field.name == "name"
        assert field.db_field == "name"
    
    def test_field_set_name_with_custom_db_field(self):
        """Test __set_name__ with custom db_field."""
        field = Field[str](db_field="custom_name")
        
        class TestModel:
            name = field
        
        assert field.name == "name"
        assert field.db_field == "custom_name"  # Should not change
    
    def test_field_get_from_class(self):
        """Test getting field from class (descriptor behavior)."""
        field = Field[str]()
        
        class TestModel:
            name = field
        
        # Getting from class should return the field itself
        assert TestModel.name is field
    
    def test_field_get_from_instance_with_value(self):
        """Test getting field value from instance."""
        field = Field[str]()
        
        class TestModel:
            name = field
        
        instance = TestModel()
        instance.__dict__["name"] = "John"
        
        assert instance.name == "John"
    
    def test_field_get_from_instance_with_default(self):
        """Test getting default value from instance."""
        field = Field[str](default="default_value")
        
        class TestModel:
            name = field
        
        instance = TestModel()
        
        assert instance.name == "default_value"
    
    def test_field_get_from_instance_with_factory(self):
        """Test getting value from default_factory."""
        def factory():
            return "factory_value"
        
        field = Field[str](default_factory=factory, required=False)
        
        class TestModel:
            name = field
        
        instance = TestModel()
        
        # First call should create and cache the value
        assert instance.name == "factory_value"
        # Should be stored in instance dict
        assert instance.__dict__["name"] == "factory_value"
    
    def test_field_get_from_instance_none(self):
        """Test getting None value from instance."""
        field = Field[str](required=False)
        
        class TestModel:
            name = field
        
        instance = TestModel()
        
        assert instance.name is None
    
    def test_field_set_valid_value(self):
        """Test setting valid field value."""
        field = Field[str]()
        
        class TestModel:
            name = field
        
        instance = TestModel()
        instance.name = "John"
        
        assert instance.__dict__["name"] == "John"
    
    def test_field_set_none_value_required_field(self):
        """Test setting None on required field raises error."""
        field = Field[str](required=True)
        
        class TestModel:
            name = field
        
        instance = TestModel()
        
        with pytest.raises(ValueError, match="Field name is required"):
            instance.name = None
    
    def test_field_set_none_value_optional_field(self):
        """Test setting None on optional field."""
        field = Field[str](required=False)
        
        class TestModel:
            name = field
        
        instance = TestModel()
        instance.name = None
        
        assert instance.__dict__["name"] is None


class TestFieldUpdateOperations:
    """Test Field update operations."""
    
    def test_field_set_operation(self):
        """Test field set operation."""
        field = Field[str]()
        field.name = "name"
        
        expr = field.set("New Value")
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "set"
        assert expr.value == "New Value"
        assert expr.field == "name"
    
    def test_field_unset_operation(self):
        """Test field unset operation."""
        field = Field[str]()
        field.name = "name"
        
        expr = field.unset()
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "unset"
        assert expr.value == ""
        assert expr.field == "name"
    
    def test_field_rename_operation(self):
        """Test field rename operation."""
        field = Field[str]()
        field.name = "old_name"
        
        expr = field.rename("new_name")
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "rename"
        assert expr.value == "new_name"
        assert expr.field == "old_name"


class TestIntField:
    """Test IntField functionality."""
    
    def test_int_field_initialization(self):
        """Test IntField initialization."""
        field = IntField(default=42)
        assert field.default == 42
        assert isinstance(field, Field)
    
    def test_int_field_inc_default(self):
        """Test inc operation with default value."""
        field = IntField()
        field.name = "counter"
        
        expr = field.inc()
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "inc"
        assert expr.value == 1
        assert expr.field == "counter"
    
    def test_int_field_inc_custom(self):
        """Test inc operation with custom value."""
        field = IntField()
        field.name = "counter"
        
        expr = field.inc(5)
        assert expr.value == 5
    
    def test_int_field_mul(self):
        """Test mul operation."""
        field = IntField()
        field.name = "value"
        
        expr = field.mul(2)
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "mul"
        assert expr.value == 2
    
    def test_int_field_min(self):
        """Test min operation."""
        field = IntField()
        field.name = "value"
        
        expr = field.min(10)
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "min"
        assert expr.value == 10
    
    def test_int_field_max(self):
        """Test max operation."""
        field = IntField()
        field.name = "value"
        
        expr = field.max(100)
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "max"
        assert expr.value == 100


class TestFloatField:
    """Test FloatField functionality."""
    
    def test_float_field_initialization(self):
        """Test FloatField initialization."""
        field = FloatField(default=3.14)
        assert field.default == 3.14
        assert isinstance(field, Field)
    
    def test_float_field_inc(self):
        """Test inc operation with float."""
        field = FloatField()
        field.name = "price"
        
        expr = field.inc(0.5)
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "inc"
        assert expr.value == 0.5
    
    def test_float_field_mul(self):
        """Test mul operation with float."""
        field = FloatField()
        field.name = "price"
        
        expr = field.mul(1.1)
        assert expr.operator == "mul"
        assert expr.value == 1.1
    
    def test_float_field_min(self):
        """Test min operation with float."""
        field = FloatField()
        field.name = "price"
        
        expr = field.min(0.0)
        assert expr.operator == "min"
        assert expr.value == 0.0
    
    def test_float_field_max(self):
        """Test max operation with float."""
        field = FloatField()
        field.name = "price"
        
        expr = field.max(999.99)
        assert expr.operator == "max"
        assert expr.value == 999.99


class TestStringField:
    """Test StringField functionality."""
    
    def test_string_field_initialization(self):
        """Test StringField initialization with constraints."""
        field = StringField(
            default="test",
            max_length=100,
            min_length=5
        )
        
        assert field.default == "test"
        assert field.max_length == 100
        assert field.min_length == 5
        assert isinstance(field, Field)
    
    def test_string_field_regex(self):
        """Test regex operation."""
        field = StringField()
        field.name = "email"
        
        expr = field.regex(r".*@.*\.com$")
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "regex"
        assert expr.value == r".*@.*\.com$"
        assert expr.field == "email"


class TestBoolField:
    """Test BoolField functionality."""
    
    def test_bool_field_initialization(self):
        """Test BoolField initialization."""
        field = BoolField(default=True)
        assert field.default is True
        assert isinstance(field, Field)
    
    def test_bool_field_toggle(self):
        """Test toggle operation."""
        field = BoolField()
        field.name = "active"
        
        expr = field.toggle()
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "toggle"
        assert expr.value is None
        assert expr.field == "active"


class TestListField:
    """Test ListField functionality."""
    
    def test_list_field_initialization(self):
        """Test ListField initialization."""
        field = ListField[str](item_type=str)
        assert field.item_type == str
        assert field.default == []
        assert isinstance(field, Field)
    
    def test_list_field_initialization_with_default(self):
        """Test ListField with custom default."""
        field = ListField[str](item_type=str, default=["initial"])
        assert field.default == ["initial"]
    
    def test_list_field_contains_all(self):
        """Test contains_all operation."""
        field = ListField[str](item_type=str)
        field.name = "tags"
        
        expr = field.contains_all(["tag1", "tag2"])
        assert isinstance(expr, QueryExpression)
        assert expr.operator == "contains_all"
        assert expr.value == ["tag1", "tag2"]
    
    def test_list_field_contains_any(self):
        """Test contains_any operation."""
        field = ListField[str](item_type=str)
        field.name = "tags"
        
        expr = field.contains_any(["tag1", "tag3"])
        assert expr.operator == "contains_any"
        assert expr.value == ["tag1", "tag3"]
    
    def test_list_field_push_basic(self):
        """Test basic push operation."""
        field = ListField[str](item_type=str)
        field.name = "tags"
        
        expr = field.push("new_tag")
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "push"
        assert expr.value == "new_tag"
        assert expr.modifiers is None
    
    def test_list_field_push_with_modifiers(self):
        """Test push operation with modifiers."""
        field = ListField[str](item_type=str)
        field.name = "tags"
        
        # Test with position
        expr = field.push("first_tag", position=0)
        assert expr.modifiers == {"$position": 0}
        
        # Test with slice
        expr = field.push("tag", slice=5)
        assert expr.modifiers == {"$slice": 5}
        
        # Test with sort
        expr = field.push("tag", sort=1)
        assert expr.modifiers == {"$sort": 1}
        
        # Test with multiple modifiers
        expr = field.push("tag", position=0, slice=10, sort=-1)
        expected = {"$position": 0, "$slice": 10, "$sort": -1}
        assert expr.modifiers == expected
    
    def test_list_field_push_all(self):
        """Test push_all operation."""
        field = ListField[str](item_type=str)
        field.name = "tags"
        
        expr = field.push_all(["tag1", "tag2", "tag3"])
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "push"
        assert expr.value == ["tag1", "tag2", "tag3"]
        assert expr.modifiers == {"$each": True}
    
    def test_list_field_pull(self):
        """Test pull operation."""
        field = ListField[str](item_type=str)
        field.name = "tags"
        
        expr = field.pull("old_tag")
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "pull"
        assert expr.value == "old_tag"
    
    def test_list_field_pull_all(self):
        """Test pull_all operation."""
        field = ListField[str](item_type=str)
        field.name = "tags"
        
        expr = field.pull_all(["old_tag1", "old_tag2"])
        assert expr.operator == "pullAll"
        assert expr.value == ["old_tag1", "old_tag2"]
    
    def test_list_field_add_to_set(self):
        """Test add_to_set operation."""
        field = ListField[str](item_type=str)
        field.name = "unique_tags"
        
        expr = field.add_to_set("unique_tag")
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "addToSet"
        assert expr.value == "unique_tag"
    
    def test_list_field_add_to_set_each(self):
        """Test add_to_set_each operation."""
        field = ListField[str](item_type=str)
        field.name = "unique_tags"
        
        expr = field.add_to_set_each(["tag1", "tag2"])
        assert expr.operator == "addToSet"
        assert expr.value == ["tag1", "tag2"]
        assert expr.modifiers == {"$each": True}
    
    def test_list_field_pop_last(self):
        """Test pop last element (default)."""
        field = ListField[str](item_type=str)
        field.name = "queue"
        
        expr = field.pop()
        assert isinstance(expr, UpdateExpression)
        assert expr.operator == "pop"
        assert expr.value == 1
    
    def test_list_field_pop_first(self):
        """Test pop first element."""
        field = ListField[str](item_type=str)
        field.name = "queue"
        
        expr = field.pop(-1)
        assert expr.value == -1
    
    def test_list_field_pop_invalid_position(self):
        """Test pop with invalid position."""
        field = ListField[str](item_type=str)
        field.name = "queue"
        
        with pytest.raises(ValueError, match="Position must be -1 \\(first\\) or 1 \\(last\\)"):
            field.pop(0)


class TestDictField:
    """Test DictField functionality."""
    
    def test_dict_field_initialization(self):
        """Test DictField initialization."""
        field = DictField(default={"key": "value"})
        assert field.default == {"key": "value"}
        assert isinstance(field, Field)
    
    def test_dict_field_get_nested(self):
        """Test get_nested method."""
        field = DictField()
        field.name = "metadata"
        
        nested = field.get_nested("user.name")
        assert isinstance(nested, NestedFieldProxy)
        assert nested.path == "metadata.user.name"
    
    def test_dict_field_get_nested_with_db_field(self):
        """Test get_nested with custom db_field."""
        field = DictField(db_field="meta")
        
        nested = field.get_nested("settings.theme")
        assert nested.path == "meta.settings.theme"
    
    def test_dict_field_set_field(self):
        """Test set_field operation."""
        field = DictField()
        field.name = "config"
        
        expr = field.set_field("user.preferences.theme", "dark")
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "config.user.preferences.theme"
        assert expr.operator == "set"
        assert expr.value == "dark"
    
    def test_dict_field_unset_field(self):
        """Test unset_field operation."""
        field = DictField()
        field.name = "config"
        
        expr = field.unset_field("user.old_setting")
        assert expr.field == "config.user.old_setting"
        assert expr.operator == "unset"
        assert expr.value == ""
    
    def test_dict_field_inc_field(self):
        """Test inc_field operation."""
        field = DictField()
        field.name = "config"
        
        expr = field.inc_field("stats.view_count", 1)
        assert expr.field == "config.stats.view_count"
        assert expr.operator == "inc"
        assert expr.value == 1


class TestNestedFieldProxy:
    """Test NestedFieldProxy functionality."""
    
    def test_nested_field_proxy_initialization(self):
        """Test NestedFieldProxy initialization."""
        proxy = NestedFieldProxy("user.settings.theme")
        assert proxy.path == "user.settings.theme"
    
    def test_nested_field_proxy_eq(self):
        """Test equality operator."""
        proxy = NestedFieldProxy("user.age")
        
        expr = proxy == 25
        assert isinstance(expr, QueryExpression)
        assert expr.field == "user.age"
        assert expr.operator == "eq"
        assert expr.value == 25
    
    def test_nested_field_proxy_ne(self):
        """Test not equal operator."""
        proxy = NestedFieldProxy("user.age")
        
        expr = proxy != 30
        assert expr.operator == "ne"
        assert expr.value == 30
    
    def test_nested_field_proxy_comparison_operators(self):
        """Test comparison operators."""
        proxy = NestedFieldProxy("user.age")
        
        lt_expr = proxy < 18
        assert lt_expr.operator == "lt"
        assert lt_expr.value == 18
        
        lte_expr = proxy <= 21
        assert lte_expr.operator == "lte"
        
        gt_expr = proxy > 65
        assert gt_expr.operator == "gt"
        
        gte_expr = proxy >= 18
        assert gte_expr.operator == "gte"


class TestFieldEdgeCases:
    """Test edge cases and error conditions."""
    
    def test_field_operations_without_name_or_db_field(self):
        """Test field operations when name and db_field are not set."""
        field = Field[str]()
        # Don't set name, db_field is also None
        
        expr = field == "test"
        # Should use empty string for field name
        assert expr.field == ""
    
    def test_field_operations_with_db_field_no_name(self):
        """Test field operations with db_field set but no name."""
        field = Field[str](db_field="custom_field")
        field.name = None
        
        expr = field == "test"
        assert expr.field == "custom_field"
    
    def test_list_field_push_no_modifiers(self):
        """Test ListField push with no modifiers results in None."""
        field = ListField[str](item_type=str)
        field.name = "tags"
        
        # All modifier parameters are None
        expr = field.push("tag", position=None, slice=None, sort=None)
        # Should result in modifiers=None since empty dict becomes None
        assert expr.modifiers is None
