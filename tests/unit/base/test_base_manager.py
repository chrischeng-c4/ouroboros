"""Tests for base manager module."""

import pytest

from data_bridge.base.fields import Field, QueryExpression
from data_bridge.base.manager import BaseManager
from data_bridge.base.model import BaseModel
from data_bridge.base.query import BaseQuery


class MockTestModel(BaseModel):
    """Test model for manager testing."""
    name = Field(required=True)
    age = Field(default=25, required=False)
    email = Field(required=True)
    active = Field(default=True, required=False)


class MockQuery(BaseQuery[MockTestModel]):
    """Mock query implementation for testing."""
    
    def filter(self, *expressions) -> "MockQuery":
        return self
    
    def limit(self, n: int) -> "MockQuery":
        return self
    
    def skip(self, n: int) -> "MockQuery":
        return self
    
    def sort(self, *fields) -> "MockQuery":
        return self
    
    def select(self, *fields) -> "MockQuery":
        return self


class MockManager(BaseManager[MockTestModel]):
    """Mock manager implementation for testing."""
    
    def find(self, *expressions) -> MockQuery:
        """Create a query with the given expressions."""
        return MockQuery(self.model_class, expressions)
    
    def all(self) -> MockQuery:
        """Return a query for all documents."""
        return MockQuery(self.model_class, [])


class TestBaseManager:
    """Test BaseManager class."""
    
    def test_initialization(self) -> None:
        """Test BaseManager initialization."""
        manager = MockManager(MockTestModel)
        assert manager.model_class is MockTestModel
    
    def test_find_method(self) -> None:
        """Test find method returns query with expressions."""
        manager = MockManager(MockTestModel)
        expr1 = QueryExpression(field="name", operator="eq", value="test")
        expr2 = QueryExpression(field="age", operator="gt", value=18)
        
        query = manager.find(expr1, expr2)
        
        assert isinstance(query, MockQuery)
        assert query.model_class is MockTestModel
        assert len(query.expressions) == 2
        assert query.expressions[0] == expr1
        assert query.expressions[1] == expr2
    
    def test_find_method_no_expressions(self) -> None:
        """Test find method with no expressions."""
        manager = MockManager(MockTestModel)
        
        query = manager.find()
        
        assert isinstance(query, MockQuery)
        assert query.model_class is MockTestModel
        assert len(query.expressions) == 0
    
    def test_all_method(self) -> None:
        """Test all method returns query for all documents."""
        manager = MockManager(MockTestModel)
        
        query = manager.all()
        
        assert isinstance(query, MockQuery)
        assert query.model_class is MockTestModel
        assert len(query.expressions) == 0
    
    def test_create_field_expressions_valid_fields(self) -> None:
        """Test _create_field_expressions with valid field names."""
        manager = MockManager(MockTestModel)
        
        expressions = manager._create_field_expressions(
            name="John", 
            age=30, 
            email="john@example.com",
            active=True
        )
        
        assert len(expressions) == 4
        
        # Check each expression
        name_expr = expressions[0]
        assert isinstance(name_expr, QueryExpression)
        assert name_expr.field == "name"
        assert name_expr.operator == "eq"
        assert name_expr.value == "John"
        
        age_expr = expressions[1]
        assert isinstance(age_expr, QueryExpression)
        assert age_expr.field == "age"
        assert age_expr.operator == "eq"
        assert age_expr.value == 30
        
        email_expr = expressions[2]
        assert isinstance(email_expr, QueryExpression)
        assert email_expr.field == "email"
        assert email_expr.operator == "eq"
        assert email_expr.value == "john@example.com"
        
        active_expr = expressions[3]
        assert isinstance(active_expr, QueryExpression)
        assert active_expr.field == "active"
        assert active_expr.operator == "eq"
        assert active_expr.value is True
    
    def test_create_field_expressions_single_field(self) -> None:
        """Test _create_field_expressions with single field."""
        manager = MockManager(MockTestModel)
        
        expressions = manager._create_field_expressions(name="John")
        
        assert len(expressions) == 1
        expr = expressions[0]
        assert isinstance(expr, QueryExpression)
        assert expr.field == "name"
        assert expr.operator == "eq"
        assert expr.value == "John"
    
    def test_create_field_expressions_no_fields(self) -> None:
        """Test _create_field_expressions with no fields."""
        manager = MockManager(MockTestModel)
        
        expressions = manager._create_field_expressions()
        
        assert len(expressions) == 0
        assert expressions == []
    
    def test_create_field_expressions_invalid_field(self) -> None:
        """Test _create_field_expressions with invalid field name."""
        manager = MockManager(MockTestModel)
        
        with pytest.raises(ValueError, match="Unknown field: invalid_field"):
            manager._create_field_expressions(invalid_field="value")
    
    def test_create_field_expressions_mixed_valid_invalid(self) -> None:
        """Test _create_field_expressions with mix of valid and invalid fields."""
        manager = MockManager(MockTestModel)
        
        # Should fail on first invalid field encountered
        with pytest.raises(ValueError, match="Unknown field: invalid_field"):
            manager._create_field_expressions(
                name="John",  # valid
                invalid_field="value",  # invalid
                age=30  # valid but won't be reached
            )
    
    def test_create_field_expressions_various_value_types(self) -> None:
        """Test _create_field_expressions with various value types."""
        manager = MockManager(MockTestModel)
        
        expressions = manager._create_field_expressions(
            name="John",           # string
            age=30,                # int
            active=True,           # bool
            email=None             # None
        )
        
        assert len(expressions) == 4
        
        # Verify values are preserved with correct types
        values = [expr.value for expr in expressions]
        assert "John" in values
        assert 30 in values
        assert True in values
        assert None in values
    
    def test_create_field_expressions_empty_string_value(self) -> None:
        """Test _create_field_expressions with empty string value."""
        manager = MockManager(MockTestModel)
        
        expressions = manager._create_field_expressions(name="")
        
        assert len(expressions) == 1
        expr = expressions[0]
        assert expr.field == "name"
        assert expr.value == ""
    
    def test_create_field_expressions_zero_value(self) -> None:
        """Test _create_field_expressions with zero value."""
        manager = MockManager(MockTestModel)
        
        expressions = manager._create_field_expressions(age=0)
        
        assert len(expressions) == 1
        expr = expressions[0]
        assert expr.field == "age"
        assert expr.value == 0
    
    def test_create_field_expressions_false_value(self) -> None:
        """Test _create_field_expressions with False value."""
        manager = MockManager(MockTestModel)
        
        expressions = manager._create_field_expressions(active=False)
        
        assert len(expressions) == 1
        expr = expressions[0]
        assert expr.field == "active"
        assert expr.value is False
    
    def test_field_access_through_model_class(self) -> None:
        """Test that manager can access fields through model class."""
        manager = MockManager(MockTestModel)
        
        # Verify that manager has access to model fields
        assert hasattr(MockTestModel, '_fields')
        assert "name" in MockTestModel._fields
        assert "age" in MockTestModel._fields
        assert "email" in MockTestModel._fields
        assert "active" in MockTestModel._fields
        
        # Test that we can create expressions using these fields
        expressions = manager._create_field_expressions(name="test")
        assert len(expressions) == 1
        assert expressions[0].field == "name"