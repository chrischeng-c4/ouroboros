"""Tests for field update operations."""

import pytest

from data_bridge.base.fields import (
    BoolField,
    DictField,
    FloatField,
    IntField,
    ListField,
    StringField,
    UpdateExpression,
)


class TestUpdateExpression:
    """Test UpdateExpression class."""
    
    def test_update_expression_creation(self) -> None:
        """Test UpdateExpression creation."""
        expr = UpdateExpression("field", "set", "value")
        assert expr.field == "field"
        assert expr.operator == "set"
        assert expr.value == "value"
        assert expr.modifiers is None
    
    def test_update_expression_with_modifiers(self) -> None:
        """Test UpdateExpression creation with modifiers."""
        modifiers = {"$each": True}
        expr = UpdateExpression("field", "push", ["a", "b"], modifiers)
        assert expr.field == "field"
        assert expr.operator == "push"
        assert expr.value == ["a", "b"]
        assert expr.modifiers == modifiers
    
    def test_update_expression_validation_empty_field(self) -> None:
        """Test validation error for empty field."""
        with pytest.raises(ValueError, match="Field cannot be empty"):
            UpdateExpression("", "set", "value")
    
    def test_update_expression_validation_empty_operator(self) -> None:
        """Test validation error for empty operator."""
        with pytest.raises(ValueError, match="Operator cannot be empty"):
            UpdateExpression("field", "", "value")


class TestBaseFieldUpdates:
    """Test base field update operations."""
    
    def test_field_set_operation(self) -> None:
        """Test field set operation."""
        field = StringField()
        field.name = "text"
        
        expr = field.set("new_value")
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "text"
        assert expr.operator == "set"
        assert expr.value == "new_value"
    
    def test_field_set_with_db_field(self) -> None:
        """Test field set operation with db_field."""
        field = StringField(db_field="text_field")
        field.name = "text"
        
        expr = field.set("new_value")
        assert expr.field == "text_field"
        assert expr.operator == "set"
        assert expr.value == "new_value"
    
    def test_field_unset_operation(self) -> None:
        """Test field unset operation."""
        field = StringField()
        field.name = "text"
        
        expr = field.unset()
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "text"
        assert expr.operator == "unset"
        assert expr.value == ""
    
    def test_field_rename_operation(self) -> None:
        """Test field rename operation."""
        field = StringField()
        field.name = "old_name"
        
        expr = field.rename("new_name")
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "old_name"
        assert expr.operator == "rename"
        assert expr.value == "new_name"


class TestNumericFieldUpdates:
    """Test numeric field update operations."""
    
    def test_int_field_inc_operation(self) -> None:
        """Test IntField increment operation."""
        field = IntField()
        field.name = "count"
        
        expr = field.inc(5)
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "count"
        assert expr.operator == "inc"
        assert expr.value == 5
    
    def test_int_field_inc_default(self) -> None:
        """Test IntField increment with default value."""
        field = IntField()
        field.name = "count"
        
        expr = field.inc()
        assert expr.value == 1
    
    def test_int_field_mul_operation(self) -> None:
        """Test IntField multiply operation."""
        field = IntField()
        field.name = "score"
        
        expr = field.mul(2)
        assert expr.field == "score"
        assert expr.operator == "mul"
        assert expr.value == 2
    
    def test_int_field_min_operation(self) -> None:
        """Test IntField minimum operation."""
        field = IntField()
        field.name = "level"
        
        expr = field.min(10)
        assert expr.field == "level"
        assert expr.operator == "min"
        assert expr.value == 10
    
    def test_int_field_max_operation(self) -> None:
        """Test IntField maximum operation."""
        field = IntField()
        field.name = "level"
        
        expr = field.max(100)
        assert expr.field == "level"
        assert expr.operator == "max"
        assert expr.value == 100
    
    def test_float_field_inc_operation(self) -> None:
        """Test FloatField increment operation."""
        field = FloatField()
        field.name = "balance"
        
        expr = field.inc(10.5)
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "balance"
        assert expr.operator == "inc"
        assert expr.value == 10.5
    
    def test_float_field_mul_operation(self) -> None:
        """Test FloatField multiply operation."""
        field = FloatField()
        field.name = "rate"
        
        expr = field.mul(1.5)
        assert expr.field == "rate"
        assert expr.operator == "mul"
        assert expr.value == 1.5


class TestBoolFieldUpdates:
    """Test boolean field update operations."""
    
    def test_bool_field_toggle_operation(self) -> None:
        """Test BoolField toggle operation."""
        field = BoolField()
        field.name = "active"
        
        expr = field.toggle()
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "active"
        assert expr.operator == "toggle"
        assert expr.value is None


class TestListFieldUpdates:
    """Test list field update operations."""
    
    def test_list_field_push_operation(self) -> None:
        """Test ListField push operation."""
        field = ListField(str)
        field.name = "tags"
        
        expr = field.push("new_tag")
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "tags"
        assert expr.operator == "push"
        assert expr.value == "new_tag"
        assert expr.modifiers is None
    
    def test_list_field_push_with_position(self) -> None:
        """Test ListField push with position modifier."""
        field = ListField(str)
        field.name = "items"
        
        expr = field.push("item", position=0)
        assert expr.field == "items"
        assert expr.operator == "push"
        assert expr.value == "item"
        assert expr.modifiers == {"$position": 0}
    
    def test_list_field_push_with_slice(self) -> None:
        """Test ListField push with slice modifier."""
        field = ListField(str)
        field.name = "recent_items"
        
        expr = field.push("item", slice=10)
        assert expr.field == "recent_items"
        assert expr.operator == "push"
        assert expr.value == "item"
        assert expr.modifiers == {"$slice": 10}
    
    def test_list_field_push_with_sort(self) -> None:
        """Test ListField push with sort modifier."""
        field = ListField(int)
        field.name = "scores"
        
        expr = field.push(95, sort=-1)
        assert expr.field == "scores"
        assert expr.operator == "push"
        assert expr.value == 95
        assert expr.modifiers == {"$sort": -1}
    
    def test_list_field_push_all_operation(self) -> None:
        """Test ListField push_all operation."""
        field = ListField(str)
        field.name = "tags"
        
        expr = field.push_all(["tag1", "tag2", "tag3"])
        assert expr.field == "tags"
        assert expr.operator == "push"
        assert expr.value == ["tag1", "tag2", "tag3"]
        assert expr.modifiers == {"$each": True}
    
    def test_list_field_pull_operation(self) -> None:
        """Test ListField pull operation."""
        field = ListField(str)
        field.name = "tags"
        
        expr = field.pull("old_tag")
        assert expr.field == "tags"
        assert expr.operator == "pull"
        assert expr.value == "old_tag"
    
    def test_list_field_pull_all_operation(self) -> None:
        """Test ListField pull_all operation."""
        field = ListField(str)
        field.name = "tags"
        
        expr = field.pull_all(["tag1", "tag2"])
        assert expr.field == "tags"
        assert expr.operator == "pullAll"
        assert expr.value == ["tag1", "tag2"]
    
    def test_list_field_add_to_set_operation(self) -> None:
        """Test ListField add_to_set operation."""
        field = ListField(str)
        field.name = "unique_tags"
        
        expr = field.add_to_set("new_tag")
        assert expr.field == "unique_tags"
        assert expr.operator == "addToSet"
        assert expr.value == "new_tag"
    
    def test_list_field_add_to_set_each_operation(self) -> None:
        """Test ListField add_to_set_each operation."""
        field = ListField(str)
        field.name = "unique_tags"
        
        expr = field.add_to_set_each(["tag1", "tag2"])
        assert expr.field == "unique_tags"
        assert expr.operator == "addToSet"
        assert expr.value == ["tag1", "tag2"]
        assert expr.modifiers == {"$each": True}
    
    def test_list_field_pop_last_operation(self) -> None:
        """Test ListField pop last element."""
        field = ListField(str)
        field.name = "stack"
        
        expr = field.pop(1)
        assert expr.field == "stack"
        assert expr.operator == "pop"
        assert expr.value == 1
    
    def test_list_field_pop_first_operation(self) -> None:
        """Test ListField pop first element."""
        field = ListField(str)
        field.name = "queue"
        
        expr = field.pop(-1)
        assert expr.field == "queue"
        assert expr.operator == "pop"
        assert expr.value == -1
    
    def test_list_field_pop_default_operation(self) -> None:
        """Test ListField pop with default (last)."""
        field = ListField(str)
        field.name = "items"
        
        expr = field.pop()
        assert expr.value == 1
    
    def test_list_field_pop_invalid_position(self) -> None:
        """Test ListField pop with invalid position."""
        field = ListField(str)
        field.name = "items"
        
        with pytest.raises(ValueError, match="Position must be -1 \\(first\\) or 1 \\(last\\)"):
            field.pop(0)


class TestDictFieldUpdates:
    """Test dict field update operations."""
    
    def test_dict_field_set_field_operation(self) -> None:
        """Test DictField set_field operation."""
        field = DictField()
        field.name = "metadata"
        
        expr = field.set_field("config.theme", "dark")
        assert isinstance(expr, UpdateExpression)
        assert expr.field == "metadata.config.theme"
        assert expr.operator == "set"
        assert expr.value == "dark"
    
    def test_dict_field_set_field_with_db_field(self) -> None:
        """Test DictField set_field with db_field."""
        field = DictField(db_field="meta_data")
        field.name = "metadata"
        
        expr = field.set_field("settings.debug", True)
        assert expr.field == "meta_data.settings.debug"
        assert expr.operator == "set"
        assert expr.value is True
    
    def test_dict_field_unset_field_operation(self) -> None:
        """Test DictField unset_field operation."""
        field = DictField()
        field.name = "config"
        
        expr = field.unset_field("old_setting")
        assert expr.field == "config.old_setting"
        assert expr.operator == "unset"
        assert expr.value == ""
    
    def test_dict_field_inc_field_operation(self) -> None:
        """Test DictField inc_field operation."""
        field = DictField()
        field.name = "stats"
        
        expr = field.inc_field("views.count", 1)
        assert expr.field == "stats.views.count"
        assert expr.operator == "inc"
        assert expr.value == 1
    
    def test_dict_field_inc_field_float_operation(self) -> None:
        """Test DictField inc_field operation with float."""
        field = DictField()
        field.name = "metrics"
        
        expr = field.inc_field("performance.score", 2.5)
        assert expr.field == "metrics.performance.score"
        assert expr.operator == "inc"
        assert expr.value == 2.5


class TestComplexUpdateScenarios:
    """Test complex update operation scenarios."""
    
    def test_multiple_field_types_updates(self) -> None:
        """Test creating multiple update expressions from different field types."""
        str_field = StringField()
        str_field.name = "name"
        
        int_field = IntField()
        int_field.name = "count"
        
        list_field = ListField(str)
        list_field.name = "tags"
        
        # Create update expressions
        updates = [
            str_field.set("New Name"),
            int_field.inc(1),
            list_field.push("new_tag")
        ]
        
        assert len(updates) == 3
        assert all(isinstance(update, UpdateExpression) for update in updates)
        assert updates[0].operator == "set"
        assert updates[1].operator == "inc"
        assert updates[2].operator == "push"
    
    def test_field_without_name_or_db_field(self) -> None:
        """Test update operations on field without name or db_field raise error."""
        field = IntField()
        
        # Should raise ValueError because field has no name or db_field
        with pytest.raises(ValueError, match="Field cannot be empty"):
            field.inc(5)