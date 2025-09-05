"""Tests for base query module."""

import pytest

from data_bridge.base.fields import CompoundExpression, QueryExpression
from data_bridge.base.query import BaseQuery
from data_bridge.base.model import BaseModel


class MockModel(BaseModel):
    """Mock model for testing."""
    pass


class MockQuery(BaseQuery[MockModel]):
    """Mock query implementation for testing."""
    
    def filter(self, *expressions: QueryExpression | CompoundExpression) -> "MockQuery":
        """Add additional filter expressions."""
        new_query = MockQuery(self.model_class, self.expressions)
        new_query.expressions.extend(expressions)
        new_query._limit_value = self._limit_value
        new_query._skip_value = self._skip_value
        new_query._sort_fields = self._sort_fields.copy()
        new_query._projection = self._projection.copy() if self._projection else None
        return new_query
    
    def limit(self, n: int) -> "MockQuery":
        """Limit the number of results."""
        new_query = MockQuery(self.model_class, self.expressions)
        new_query._limit_value = n
        new_query._skip_value = self._skip_value
        new_query._sort_fields = self._sort_fields.copy()
        new_query._projection = self._projection.copy() if self._projection else None
        return new_query
    
    def skip(self, n: int) -> "MockQuery":
        """Skip the first n results."""
        new_query = MockQuery(self.model_class, self.expressions)
        new_query._limit_value = self._limit_value
        new_query._skip_value = n
        new_query._sort_fields = self._sort_fields.copy()
        new_query._projection = self._projection.copy() if self._projection else None
        return new_query
    
    def sort(self, *fields: str | tuple[str, int]) -> "MockQuery":
        """Sort results by field(s)."""
        new_query = MockQuery(self.model_class, self.expressions)
        new_query._limit_value = self._limit_value
        new_query._skip_value = self._skip_value
        new_query._sort_fields = self._parse_sort_fields(*fields)
        new_query._projection = self._projection.copy() if self._projection else None
        return new_query
    
    def select(self, *fields: str) -> "MockQuery":
        """Select specific fields to return (projection)."""
        new_query = MockQuery(self.model_class, self.expressions)
        new_query._limit_value = self._limit_value
        new_query._skip_value = self._skip_value
        new_query._sort_fields = self._sort_fields.copy()
        new_query._projection = list(fields)
        return new_query


class TestBaseQuery:
    """Test BaseQuery class."""
    
    def test_initialization(self) -> None:
        """Test BaseQuery initialization."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        
        query = MockQuery(MockModel, [expr1, expr2])
        
        assert query.model_class is MockModel
        assert len(query.expressions) == 2
        assert query.expressions[0] == expr1
        assert query.expressions[1] == expr2
        assert query._limit_value is None
        assert query._skip_value == 0
        assert query._sort_fields == []
        assert query._projection is None
    
    def test_initialization_empty_expressions(self) -> None:
        """Test BaseQuery initialization with empty expressions."""
        query = MockQuery(MockModel, [])
        
        assert query.model_class is MockModel
        assert len(query.expressions) == 0
        assert query._limit_value is None
        assert query._skip_value == 0
        assert query._sort_fields == []
        assert query._projection is None
    
    def test_parse_sort_fields_string_fields(self) -> None:
        """Test _parse_sort_fields with string fields."""
        query = MockQuery(MockModel, [])
        
        # Test ascending fields
        result = query._parse_sort_fields("name", "email", "created_at")
        expected = [("name", 1), ("email", 1), ("created_at", 1)]
        assert result == expected
        
        # Test descending fields (with minus prefix)
        result = query._parse_sort_fields("-name", "-created_at")
        expected = [("name", -1), ("created_at", -1)]
        assert result == expected
        
        # Test mixed fields
        result = query._parse_sort_fields("name", "-created_at", "email")
        expected = [("name", 1), ("created_at", -1), ("email", 1)]
        assert result == expected
    
    def test_parse_sort_fields_tuple_fields(self) -> None:
        """Test _parse_sort_fields with tuple fields."""
        query = MockQuery(MockModel, [])
        
        # Test tuple fields
        result = query._parse_sort_fields(("name", 1), ("created_at", -1))
        expected = [("name", 1), ("created_at", -1)]
        assert result == expected
    
    def test_parse_sort_fields_mixed(self) -> None:
        """Test _parse_sort_fields with mixed string and tuple fields."""
        query = MockQuery(MockModel, [])
        
        result = query._parse_sort_fields("name", ("age", -1), "-created_at", ("email", 1))
        expected = [("name", 1), ("age", -1), ("created_at", -1), ("email", 1)]
        assert result == expected
    
    def test_parse_sort_fields_empty(self) -> None:
        """Test _parse_sort_fields with no fields."""
        query = MockQuery(MockModel, [])
        
        result = query._parse_sort_fields()
        assert result == []
    
    def test_parse_sort_fields_single_minus(self) -> None:
        """Test _parse_sort_fields with field that is just a minus."""
        query = MockQuery(MockModel, [])
        
        # Edge case: field name that starts with minus but has more content
        result = query._parse_sort_fields("-")
        expected = [("", -1)]  # This is an edge case behavior
        assert result == expected
    
    def test_filter_method(self) -> None:
        """Test filter method creates new query with additional expressions."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        query = MockQuery(MockModel, [expr1])
        
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        new_query = query.filter(expr2)
        
        # Original query unchanged
        assert len(query.expressions) == 1
        assert query.expressions[0] == expr1
        
        # New query has both expressions
        assert len(new_query.expressions) == 2
        assert new_query.expressions[0] == expr1
        assert new_query.expressions[1] == expr2
        assert new_query.model_class is MockModel
    
    def test_filter_method_multiple_expressions(self) -> None:
        """Test filter method with multiple expressions."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        query = MockQuery(MockModel, [expr1])
        
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        expr3 = QueryExpression(field="active", operator="eq", value=True)
        new_query = query.filter(expr2, expr3)
        
        assert len(new_query.expressions) == 3
        assert new_query.expressions[0] == expr1
        assert new_query.expressions[1] == expr2
        assert new_query.expressions[2] == expr3
    
    def test_limit_method(self) -> None:
        """Test limit method."""
        query = MockQuery(MockModel, [])
        new_query = query.limit(10)
        
        assert query._limit_value is None  # Original unchanged
        assert new_query._limit_value == 10
        assert new_query.model_class is MockModel
    
    def test_skip_method(self) -> None:
        """Test skip method."""
        query = MockQuery(MockModel, [])
        new_query = query.skip(5)
        
        assert query._skip_value == 0  # Original unchanged
        assert new_query._skip_value == 5
        assert new_query.model_class is MockModel
    
    def test_sort_method(self) -> None:
        """Test sort method."""
        query = MockQuery(MockModel, [])
        new_query = query.sort("name", "-created_at")
        
        assert query._sort_fields == []  # Original unchanged
        assert new_query._sort_fields == [("name", 1), ("created_at", -1)]
        assert new_query.model_class is MockModel
    
    def test_select_method(self) -> None:
        """Test select method."""
        query = MockQuery(MockModel, [])
        new_query = query.select("name", "email", "created_at")
        
        assert query._projection is None  # Original unchanged
        assert new_query._projection == ["name", "email", "created_at"]
        assert new_query.model_class is MockModel
    
    def test_method_chaining(self) -> None:
        """Test method chaining preserves all query parameters."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        query = MockQuery(MockModel, [expr1])
        
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        final_query = (query
                       .filter(expr2)
                       .limit(10)
                       .skip(5)
                       .sort("name", "-created_at")
                       .select("name", "email"))
        
        # Check all parameters are preserved
        assert len(final_query.expressions) == 2
        assert final_query.expressions[0] == expr1
        assert final_query.expressions[1] == expr2
        assert final_query._limit_value == 10
        assert final_query._skip_value == 5
        assert final_query._sort_fields == [("name", 1), ("created_at", -1)]
        assert final_query._projection == ["name", "email"]
        assert final_query.model_class is MockModel
    
    def test_immutability(self) -> None:
        """Test that query operations don't mutate the original query."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        original_query = MockQuery(MockModel, [expr1])
        
        # Perform various operations
        original_query.filter(QueryExpression(field="age", operator="gt", value=18))
        original_query.limit(10)
        original_query.skip(5)
        original_query.sort("name")
        original_query.select("name", "email")
        
        # Original query should be unchanged
        assert len(original_query.expressions) == 1
        assert original_query.expressions[0] == expr1
        assert original_query._limit_value is None
        assert original_query._skip_value == 0
        assert original_query._sort_fields == []
        assert original_query._projection is None

    def test_parse_sort_fields_invalid_type(self) -> None:
        """Test _parse_sort_fields with invalid field types."""
        query = MockQuery(MockModel, [])
        
        # Test with None (should be skipped/ignored)
        result = query._parse_sort_fields("name", None, "email")  # type: ignore[arg-type]
        # None should be ignored, only string fields processed
        expected = [("name", 1), ("email", 1)]
        assert result == expected

    def test_parse_sort_fields_edge_cases(self) -> None:
        """Test _parse_sort_fields with various edge cases."""
        query = MockQuery(MockModel, [])
        
        # Test empty string field
        result = query._parse_sort_fields("")
        expected = [("", 1)]
        assert result == expected
        
        # Test field that is just spaces
        result = query._parse_sort_fields("  ")
        expected = [("  ", 1)]
        assert result == expected
        
        # Test field starting with minus but having spaces
        result = query._parse_sort_fields("-  name  ")
        expected = [("  name  ", -1)]
        assert result == expected

    def test_expressions_initialization_with_tuples(self) -> None:
        """Test initialization with expressions as tuples."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        
        # Test with tuple instead of list
        query = MockQuery(MockModel, (expr1, expr2))
        
        assert query.model_class is MockModel
        assert len(query.expressions) == 2
        assert isinstance(query.expressions, list)  # Should be converted to list
        assert query.expressions[0] == expr1
        assert query.expressions[1] == expr2

    def test_compound_expressions_in_query(self) -> None:
        """Test that compound expressions work in queries."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        compound_expr = CompoundExpression("and", [expr1, expr2])
        
        query = MockQuery(MockModel, [compound_expr])
        
        assert len(query.expressions) == 1
        assert query.expressions[0] == compound_expr
        assert isinstance(query.expressions[0], CompoundExpression)

    def test_filter_with_compound_expressions(self) -> None:
        """Test filter method with compound expressions."""
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        query = MockQuery(MockModel, [expr1])
        
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        expr3 = QueryExpression(field="active", operator="eq", value=True)
        compound_expr = CompoundExpression("and", [expr2, expr3])
        
        new_query = query.filter(compound_expr)
        
        assert len(new_query.expressions) == 2
        assert new_query.expressions[0] == expr1
        assert new_query.expressions[1] == compound_expr
        assert isinstance(new_query.expressions[1], CompoundExpression)