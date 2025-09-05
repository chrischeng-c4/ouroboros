"""Tests for MongoDB update translator."""

import pytest

from data_bridge.base.fields import UpdateExpression
from data_bridge.mongo.translator import MongoUpdateTranslator


class TestMongoUpdateTranslator:
    """Test MongoUpdateTranslator class."""
    
    def test_empty_expressions_list(self) -> None:
        """Test translation of empty expressions list."""
        result = MongoUpdateTranslator.translate([])
        assert result == {}
    
    def test_single_set_expression(self) -> None:
        """Test translation of single set expression."""
        expr = UpdateExpression("name", "set", "John")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$set": {"name": "John"}}
    
    def test_single_unset_expression(self) -> None:
        """Test translation of single unset expression."""
        expr = UpdateExpression("temp_field", "unset", "")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$unset": {"temp_field": ""}}
    
    def test_single_rename_expression(self) -> None:
        """Test translation of single rename expression."""
        expr = UpdateExpression("old_name", "rename", "new_name")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$rename": {"old_name": "new_name"}}
    
    def test_single_inc_expression(self) -> None:
        """Test translation of single increment expression."""
        expr = UpdateExpression("count", "inc", 5)
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$inc": {"count": 5}}
    
    def test_single_mul_expression(self) -> None:
        """Test translation of single multiply expression."""
        expr = UpdateExpression("score", "mul", 2)
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$mul": {"score": 2}}
    
    def test_single_min_expression(self) -> None:
        """Test translation of single minimum expression."""
        expr = UpdateExpression("level", "min", 10)
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$min": {"level": 10}}
    
    def test_single_max_expression(self) -> None:
        """Test translation of single maximum expression."""
        expr = UpdateExpression("level", "max", 100)
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$max": {"level": 100}}
    
    def test_single_push_expression(self) -> None:
        """Test translation of single push expression."""
        expr = UpdateExpression("tags", "push", "new_tag")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$push": {"tags": "new_tag"}}
    
    def test_single_pull_expression(self) -> None:
        """Test translation of single pull expression."""
        expr = UpdateExpression("tags", "pull", "old_tag")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$pull": {"tags": "old_tag"}}
    
    def test_single_add_to_set_expression(self) -> None:
        """Test translation of single addToSet expression."""
        expr = UpdateExpression("unique_tags", "addToSet", "unique_tag")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$addToSet": {"unique_tags": "unique_tag"}}
    
    def test_single_pop_expression(self) -> None:
        """Test translation of single pop expression."""
        expr = UpdateExpression("queue", "pop", -1)
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$pop": {"queue": -1}}
    
    def test_toggle_expression(self) -> None:
        """Test translation of toggle expression."""
        expr = UpdateExpression("active", "toggle", None)
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$set": {"active": {"$not": "$active"}}}
    
    def test_push_with_each_modifier(self) -> None:
        """Test translation of push with $each modifier."""
        expr = UpdateExpression("tags", "push", ["tag1", "tag2"], {"$each": True})
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$push": {"tags": {"$each": ["tag1", "tag2"]}}}
    
    def test_push_with_position_modifier(self) -> None:
        """Test translation of push with position modifier."""
        modifiers = {"$position": 0}
        expr = UpdateExpression("items", "push", "new_item", modifiers)
        result = MongoUpdateTranslator.translate([expr])
        expected = {
            "$push": {
                "items": {
                    "$each": ["new_item"],
                    "$position": 0
                }
            }
        }
        assert result == expected
    
    def test_push_with_slice_modifier(self) -> None:
        """Test translation of push with slice modifier."""
        modifiers = {"$slice": 10}
        expr = UpdateExpression("recent_items", "push", "item", modifiers)
        result = MongoUpdateTranslator.translate([expr])
        expected = {
            "$push": {
                "recent_items": {
                    "$each": ["item"],
                    "$slice": 10
                }
            }
        }
        assert result == expected
    
    def test_push_with_sort_modifier(self) -> None:
        """Test translation of push with sort modifier."""
        modifiers = {"$sort": -1}
        expr = UpdateExpression("scores", "push", 95, modifiers)
        result = MongoUpdateTranslator.translate([expr])
        expected = {
            "$push": {
                "scores": {
                    "$each": [95],
                    "$sort": -1
                }
            }
        }
        assert result == expected
    
    def test_push_with_multiple_modifiers(self) -> None:
        """Test translation of push with multiple modifiers."""
        modifiers = {"$position": 0, "$slice": 5, "$sort": 1}
        expr = UpdateExpression("top_scores", "push", 100, modifiers)
        result = MongoUpdateTranslator.translate([expr])
        expected = {
            "$push": {
                "top_scores": {
                    "$each": [100],
                    "$position": 0,
                    "$slice": 5,
                    "$sort": 1
                }
            }
        }
        assert result == expected
    
    def test_add_to_set_with_each_modifier(self) -> None:
        """Test translation of addToSet with $each modifier."""
        expr = UpdateExpression("unique_tags", "addToSet", ["tag1", "tag2"], {"$each": True})
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$addToSet": {"unique_tags": {"$each": ["tag1", "tag2"]}}}
    
    def test_multiple_expressions_same_operator(self) -> None:
        """Test translation of multiple expressions with same operator."""
        expr1 = UpdateExpression("name", "set", "John")
        expr2 = UpdateExpression("age", "set", 30)
        result = MongoUpdateTranslator.translate([expr1, expr2])
        assert result == {"$set": {"name": "John", "age": 30}}
    
    def test_multiple_expressions_different_operators(self) -> None:
        """Test translation of multiple expressions with different operators."""
        expr1 = UpdateExpression("name", "set", "John")
        expr2 = UpdateExpression("count", "inc", 1)
        expr3 = UpdateExpression("tags", "push", "new_tag")
        result = MongoUpdateTranslator.translate([expr1, expr2, expr3])
        
        expected = {
            "$set": {"name": "John"},
            "$inc": {"count": 1},
            "$push": {"tags": "new_tag"}
        }
        assert result == expected
    
    def test_multiple_expressions_mixed_types(self) -> None:
        """Test translation of multiple expressions with mixed operations."""
        expr1 = UpdateExpression("profile.name", "set", "Updated Name")
        expr2 = UpdateExpression("stats.views", "inc", 1)
        expr3 = UpdateExpression("tags", "push", "tag", {"$position": 0})
        expr4 = UpdateExpression("active", "toggle", None)
        
        result = MongoUpdateTranslator.translate([expr1, expr2, expr3, expr4])
        
        expected = {
            "$set": {
                "profile.name": "Updated Name",
                "active": {"$not": "$active"}
            },
            "$inc": {"stats.views": 1},
            "$push": {
                "tags": {
                    "$each": ["tag"],
                    "$position": 0
                }
            }
        }
        assert result == expected
    
    def test_unsupported_operator_error(self) -> None:
        """Test error for unsupported operator."""
        expr = UpdateExpression("field", "unsupported", "value")
        with pytest.raises(ValueError, match="Unsupported update operator: unsupported"):
            MongoUpdateTranslator.translate([expr])


class TestOperatorMapping:
    """Test operator mapping functionality."""
    
    def test_operator_mapping_completeness(self) -> None:
        """Test that operator mapping contains expected operators."""
        mapping = MongoUpdateTranslator.OPERATOR_MAPPING
        
        expected_operators = {
            "set", "unset", "rename", "inc", "mul", "min", "max",
            "push", "pull", "pullAll", "addToSet", "pop", "toggle"
        }
        assert set(mapping.keys()) == expected_operators
    
    def test_operator_mapping_values(self) -> None:
        """Test that operator mapping values are correct."""
        mapping = MongoUpdateTranslator.OPERATOR_MAPPING
        
        assert mapping["set"] == "$set"
        assert mapping["unset"] == "$unset"
        assert mapping["rename"] == "$rename"
        assert mapping["inc"] == "$inc"
        assert mapping["mul"] == "$mul"
        assert mapping["min"] == "$min"
        assert mapping["max"] == "$max"
        assert mapping["push"] == "$push"
        assert mapping["pull"] == "$pull"
        assert mapping["pullAll"] == "$pullAll"
        assert mapping["addToSet"] == "$addToSet"
        assert mapping["pop"] == "$pop"
        assert mapping["toggle"] == "toggle"  # Special case
    
    def test_mongodb_operator_passthrough(self) -> None:
        """Test that MongoDB operators are passed through unchanged."""
        expr = UpdateExpression("field", "$customOp", "value")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$customOp": {"field": "value"}}


class TestModifierHandling:
    """Test modifier handling functionality."""
    
    def test_apply_modifiers_no_modifiers(self) -> None:
        """Test _apply_modifiers with no modifiers."""
        result = MongoUpdateTranslator._apply_modifiers("value", None)
        assert result == "value"
    
    def test_apply_modifiers_each_marker(self) -> None:
        """Test _apply_modifiers with $each marker."""
        result = MongoUpdateTranslator._apply_modifiers(["a", "b"], {"$each": True})
        assert result == {"$each": ["a", "b"]}
    
    def test_apply_modifiers_each_marker_single_value(self) -> None:
        """Test _apply_modifiers with $each marker and single value."""
        result = MongoUpdateTranslator._apply_modifiers("value", {"$each": True})
        assert result == {"$each": ["value"]}
    
    def test_apply_modifiers_positioning(self) -> None:
        """Test _apply_modifiers with positioning modifiers."""
        modifiers = {"$position": 0, "$slice": 10}
        result = MongoUpdateTranslator._apply_modifiers("value", modifiers)
        expected = {
            "$each": ["value"],
            "$position": 0,
            "$slice": 10
        }
        assert result == expected


class TestEdgeCases:
    """Test edge cases and special values."""
    
    def test_translate_with_none_value(self) -> None:
        """Test translation with None value."""
        expr = UpdateExpression("optional_field", "set", None)
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$set": {"optional_field": None}}
    
    def test_translate_with_boolean_values(self) -> None:
        """Test translation with boolean values."""
        expr_true = UpdateExpression("active", "set", True)
        result_true = MongoUpdateTranslator.translate([expr_true])
        assert result_true == {"$set": {"active": True}}
        
        expr_false = UpdateExpression("active", "set", False)
        result_false = MongoUpdateTranslator.translate([expr_false])
        assert result_false == {"$set": {"active": False}}
    
    def test_translate_with_zero_values(self) -> None:
        """Test translation with zero values."""
        expr_int = UpdateExpression("count", "set", 0)
        result_int = MongoUpdateTranslator.translate([expr_int])
        assert result_int == {"$set": {"count": 0}}
        
        expr_float = UpdateExpression("balance", "set", 0.0)
        result_float = MongoUpdateTranslator.translate([expr_float])
        assert result_float == {"$set": {"balance": 0.0}}
    
    def test_translate_with_empty_string(self) -> None:
        """Test translation with empty string value."""
        expr = UpdateExpression("description", "set", "")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$set": {"description": ""}}
    
    def test_translate_with_list_values(self) -> None:
        """Test translation with list values."""
        expr = UpdateExpression("tags", "set", ["python", "mongodb"])
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$set": {"tags": ["python", "mongodb"]}}
    
    def test_translate_with_dict_values(self) -> None:
        """Test translation with dictionary values."""
        value = {"nested": "object", "count": 42}
        expr = UpdateExpression("metadata", "set", value)
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$set": {"metadata": {"nested": "object", "count": 42}}}
    
    def test_nested_field_operations(self) -> None:
        """Test operations on nested fields."""
        expr = UpdateExpression("user.profile.name", "set", "John Doe")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$set": {"user.profile.name": "John Doe"}}
    
    def test_array_field_with_dot_notation(self) -> None:
        """Test array operations with dot notation."""
        expr = UpdateExpression("users.0.name", "set", "First User")
        result = MongoUpdateTranslator.translate([expr])
        assert result == {"$set": {"users.0.name": "First User"}}