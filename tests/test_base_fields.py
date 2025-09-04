"""Tests for base fields module."""



from data_bridge.base.fields import CompoundExpression, Field, QueryExpression


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
