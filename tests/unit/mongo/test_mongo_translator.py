"""Tests for MongoDB query translator."""

import pytest

from data_bridge.base.fields import CompoundExpression, QueryExpression
from data_bridge.mongo.translator import MongoQueryTranslator


class TestMongoQueryTranslator:
    """Test MongoQueryTranslator class."""
    
    def test_empty_expressions_list(self) -> None:
        """Test translation of empty expressions list."""
        result = MongoQueryTranslator.translate([])
        assert result == {}
    
    def test_single_equality_expression(self) -> None:
        """Test translation of single equality expression."""
        expr = QueryExpression(field="name", operator="eq", value="John")
        result = MongoQueryTranslator.translate([expr])
        assert result == {"name": "John"}
    
    def test_single_not_equal_expression(self) -> None:
        """Test translation of single not equal expression."""
        expr = QueryExpression(field="age", operator="ne", value=25)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"age": {"$ne": 25}}
    
    def test_single_greater_than_expression(self) -> None:
        """Test translation of single greater than expression."""
        expr = QueryExpression(field="age", operator="gt", value=18)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"age": {"$gt": 18}}
    
    def test_single_greater_than_equal_expression(self) -> None:
        """Test translation of single greater than or equal expression."""
        expr = QueryExpression(field="age", operator="gte", value=18)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"age": {"$gte": 18}}
    
    def test_single_less_than_expression(self) -> None:
        """Test translation of single less than expression."""
        expr = QueryExpression(field="age", operator="lt", value=65)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"age": {"$lt": 65}}
    
    def test_single_less_than_equal_expression(self) -> None:
        """Test translation of single less than or equal expression."""
        expr = QueryExpression(field="age", operator="lte", value=65)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"age": {"$lte": 65}}
    
    def test_single_in_expression(self) -> None:
        """Test translation of single in expression."""
        expr = QueryExpression(field="status", operator="in", value=["active", "pending"])
        result = MongoQueryTranslator.translate([expr])
        assert result == {"status": {"$in": ["active", "pending"]}}
    
    def test_single_not_in_expression(self) -> None:
        """Test translation of single not in expression.""" 
        expr = QueryExpression(field="status", operator="nin", value=["deleted", "banned"])
        result = MongoQueryTranslator.translate([expr])
        assert result == {"status": {"$nin": ["deleted", "banned"]}}
    
    def test_single_regex_expression(self) -> None:
        """Test translation of single regex expression."""
        expr = QueryExpression(field="email", operator="regex", value=r".*@example\.com$")
        result = MongoQueryTranslator.translate([expr])
        assert result == {"email": {"$regex": r".*@example\.com$"}}
    
    def test_single_exists_expression(self) -> None:
        """Test translation of single exists expression."""
        expr = QueryExpression(field="optional_field", operator="exists", value=True)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"optional_field": {"$exists": True}}
        
        expr_false = QueryExpression(field="optional_field", operator="exists", value=False)
        result_false = MongoQueryTranslator.translate([expr_false])
        assert result_false == {"optional_field": {"$exists": False}}
    
    def test_single_size_expression(self) -> None:
        """Test translation of single size expression."""
        expr = QueryExpression(field="tags", operator="size", value=3)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"tags": {"$size": 3}}
    
    def test_unsupported_operator_error(self) -> None:
        """Test error for unsupported operator."""
        expr = QueryExpression(field="field", operator="unsupported", value="value")
        with pytest.raises(ValueError, match="Unsupported operator: unsupported"):
            MongoQueryTranslator.translate([expr])
    
    def test_multiple_expressions_implicit_and(self) -> None:
        """Test translation of multiple expressions with implicit AND."""
        expr1 = QueryExpression(field="name", operator="eq", value="John")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        result = MongoQueryTranslator.translate([expr1, expr2])
        
        expected = {
            "$and": [
                {"name": "John"},
                {"age": {"$gt": 18}}
            ]
        }
        assert result == expected
    
    def test_multiple_expressions_mixed_operators(self) -> None:
        """Test translation of multiple expressions with mixed operators."""
        expr1 = QueryExpression(field="name", operator="eq", value="John")
        expr2 = QueryExpression(field="age", operator="gte", value=18)
        expr3 = QueryExpression(field="status", operator="in", value=["active", "pending"])
        result = MongoQueryTranslator.translate([expr1, expr2, expr3])
        
        expected = {
            "$and": [
                {"name": "John"},
                {"age": {"$gte": 18}},
                {"status": {"$in": ["active", "pending"]}}
            ]
        }
        assert result == expected


class TestCompoundExpression:
    """Test compound expression translation."""
    
    def test_compound_and_expression(self) -> None:
        """Test translation of compound AND expression."""
        expr1 = QueryExpression(field="name", operator="eq", value="John")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        compound = CompoundExpression("and", [expr1, expr2])
        
        result = MongoQueryTranslator.translate([compound])
        expected = {
            "$and": [
                {"name": "John"},
                {"age": {"$gt": 18}}
            ]
        }
        assert result == expected
    
    def test_compound_or_expression(self) -> None:
        """Test translation of compound OR expression."""
        expr1 = QueryExpression(field="name", operator="eq", value="John")
        expr2 = QueryExpression(field="name", operator="eq", value="Jane")
        compound = CompoundExpression("or", [expr1, expr2])
        
        result = MongoQueryTranslator.translate([compound])
        expected = {
            "$or": [
                {"name": "John"},
                {"name": "Jane"}
            ]
        }
        assert result == expected
    
    def test_compound_not_expression(self) -> None:
        """Test translation of compound NOT expression."""
        expr = QueryExpression(field="name", operator="eq", value="John")
        compound = CompoundExpression("not", [expr])
        
        result = MongoQueryTranslator.translate([compound])
        expected = {
            "$not": {"name": "John"}
        }
        assert result == expected
    
    def test_compound_not_expression_multiple_operands_error(self) -> None:
        """Test error for NOT expression with multiple operands."""
        expr1 = QueryExpression(field="name", operator="eq", value="John")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        compound = CompoundExpression("not", [expr1, expr2])
        
        with pytest.raises(ValueError, match="NOT operator must have exactly one operand"):
            MongoQueryTranslator.translate([compound])
    
    def test_nested_compound_expressions(self) -> None:
        """Test translation of nested compound expressions."""
        # (name = "John" OR name = "Jane") AND age > 18
        expr1 = QueryExpression(field="name", operator="eq", value="John")
        expr2 = QueryExpression(field="name", operator="eq", value="Jane")
        or_compound = CompoundExpression("or", [expr1, expr2])
        
        expr3 = QueryExpression(field="age", operator="gt", value=18)
        and_compound = CompoundExpression("and", [or_compound, expr3])
        
        result = MongoQueryTranslator.translate([and_compound])
        expected = {
            "$and": [
                {
                    "$or": [
                        {"name": "John"},
                        {"name": "Jane"}
                    ]
                },
                {"age": {"$gt": 18}}
            ]
        }
        assert result == expected
    
    def test_complex_nested_compound_expressions(self) -> None:
        """Test translation of complex nested compound expressions."""
        # NOT ((name = "John" OR name = "Jane") AND age > 18)
        expr1 = QueryExpression(field="name", operator="eq", value="John")
        expr2 = QueryExpression(field="name", operator="eq", value="Jane")
        or_compound = CompoundExpression("or", [expr1, expr2])
        
        expr3 = QueryExpression(field="age", operator="gt", value=18)
        and_compound = CompoundExpression("and", [or_compound, expr3])
        
        not_compound = CompoundExpression("not", [and_compound])
        
        result = MongoQueryTranslator.translate([not_compound])
        expected = {
            "$not": {
                "$and": [
                    {
                        "$or": [
                            {"name": "John"},
                            {"name": "Jane"}
                        ]
                    },
                    {"age": {"$gt": 18}}
                ]
            }
        }
        assert result == expected
    
    def test_unsupported_compound_operator_error(self) -> None:
        """Test error for unsupported compound operator."""
        expr = QueryExpression(field="name", operator="eq", value="John")
        compound = CompoundExpression("unsupported", [expr])
        
        with pytest.raises(ValueError, match="Unsupported compound operator: unsupported"):
            MongoQueryTranslator.translate([compound])
    
    def test_unknown_expression_type_error(self) -> None:
        """Test error for unknown expression type."""
        # Create a mock object that's neither QueryExpression nor CompoundExpression
        class UnknownExpression:
            pass
        
        unknown_expr = UnknownExpression()
        
        with pytest.raises(ValueError, match="Unknown expression type"):
            MongoQueryTranslator._translate_single(unknown_expr)  # type: ignore


class TestSortTranslation:
    """Test sort field translation."""
    
    def test_translate_sort_empty(self) -> None:
        """Test translation of empty sort fields."""
        result = MongoQueryTranslator.translate_sort([])
        assert result == []
    
    def test_translate_sort_single_field(self) -> None:
        """Test translation of single sort field."""
        sort_fields = [("name", 1)]
        result = MongoQueryTranslator.translate_sort(sort_fields)
        assert result == [("name", 1)]
    
    def test_translate_sort_multiple_fields(self) -> None:
        """Test translation of multiple sort fields."""
        sort_fields = [("name", 1), ("age", -1), ("created_at", -1)]
        result = MongoQueryTranslator.translate_sort(sort_fields)
        assert result == [("name", 1), ("age", -1), ("created_at", -1)]
    
    def test_translate_sort_ascending_descending(self) -> None:
        """Test translation of ascending and descending sort fields."""
        sort_fields = [("name", 1), ("age", -1)]
        result = MongoQueryTranslator.translate_sort(sort_fields)
        assert result == [("name", 1), ("age", -1)]


class TestProjectionTranslation:
    """Test field projection translation."""
    
    def test_translate_projection_empty(self) -> None:
        """Test translation of empty projection fields."""
        result = MongoQueryTranslator.translate_projection([])
        assert result == {}
    
    def test_translate_projection_single_field(self) -> None:
        """Test translation of single projection field."""
        fields = ["name"]
        result = MongoQueryTranslator.translate_projection(fields)
        assert result == {"name": 1}
    
    def test_translate_projection_multiple_fields(self) -> None:
        """Test translation of multiple projection fields."""
        fields = ["name", "email", "age"]
        result = MongoQueryTranslator.translate_projection(fields)
        assert result == {"name": 1, "email": 1, "age": 1}
    
    def test_translate_projection_field_order_preserved(self) -> None:
        """Test that projection field order is preserved in dict."""
        fields = ["z_field", "a_field", "m_field"]
        result = MongoQueryTranslator.translate_projection(fields)
        
        # Python 3.7+ preserves dict order
        expected = {"z_field": 1, "a_field": 1, "m_field": 1}
        assert result == expected
        assert list(result.keys()) == fields


class TestOperatorMapping:
    """Test operator mapping constants."""
    
    def test_operator_mapping_completeness(self) -> None:
        """Test that operator mapping contains expected operators."""
        mapping = MongoQueryTranslator.OPERATOR_MAPPING
        
        # Test all expected operators are present
        expected_operators = {
            "eq", "ne", "gt", "gte", "lt", "lte", 
            "in", "nin", "regex", "exists", "size"
        }
        assert set(mapping.keys()) == expected_operators
    
    def test_operator_mapping_values(self) -> None:
        """Test that operator mapping values are correct."""
        mapping = MongoQueryTranslator.OPERATOR_MAPPING
        
        assert mapping["eq"] is None  # Implicit equality
        assert mapping["ne"] == "$ne"
        assert mapping["gt"] == "$gt"
        assert mapping["gte"] == "$gte"
        assert mapping["lt"] == "$lt"
        assert mapping["lte"] == "$lte"
        assert mapping["in"] == "$in"
        assert mapping["nin"] == "$nin"
        assert mapping["regex"] == "$regex"
        assert mapping["exists"] == "$exists"
        assert mapping["size"] == "$size"


class TestEdgeCases:
    """Test edge cases and special values."""
    
    def test_translate_with_none_value(self) -> None:
        """Test translation with None value."""
        expr = QueryExpression(field="optional_field", operator="eq", value=None)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"optional_field": None}
    
    def test_translate_with_boolean_values(self) -> None:
        """Test translation with boolean values."""
        expr_true = QueryExpression(field="active", operator="eq", value=True)
        result_true = MongoQueryTranslator.translate([expr_true])
        assert result_true == {"active": True}
        
        expr_false = QueryExpression(field="active", operator="eq", value=False)
        result_false = MongoQueryTranslator.translate([expr_false])
        assert result_false == {"active": False}
    
    def test_translate_with_zero_values(self) -> None:
        """Test translation with zero values."""
        expr_int = QueryExpression(field="count", operator="eq", value=0)
        result_int = MongoQueryTranslator.translate([expr_int])
        assert result_int == {"count": 0}
        
        expr_float = QueryExpression(field="balance", operator="eq", value=0.0)
        result_float = MongoQueryTranslator.translate([expr_float])
        assert result_float == {"balance": 0.0}
    
    def test_translate_with_empty_string(self) -> None:
        """Test translation with empty string value."""
        expr = QueryExpression(field="description", operator="eq", value="")
        result = MongoQueryTranslator.translate([expr])
        assert result == {"description": ""}
    
    def test_translate_with_list_values(self) -> None:
        """Test translation with list values."""
        expr = QueryExpression(field="tags", operator="eq", value=["python", "mongodb"])
        result = MongoQueryTranslator.translate([expr])
        assert result == {"tags": ["python", "mongodb"]}
    
    def test_translate_with_dict_values(self) -> None:
        """Test translation with dictionary values."""
        value = {"nested": "object", "count": 42}
        expr = QueryExpression(field="metadata", operator="eq", value=value)
        result = MongoQueryTranslator.translate([expr])
        assert result == {"metadata": {"nested": "object", "count": 42}}