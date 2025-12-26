"""
Unit tests for QueryBuilder class.

Tests the query builder Python API without requiring a real database connection.
"""
import pytest
from data_bridge.postgres import Table, Column
from data_bridge.postgres.query import QueryBuilder
from data_bridge.postgres.columns import SqlExpr, ColumnProxy


@pytest.fixture
def User():
    """Sample User table for query tests."""

    class User(Table):
        name: str
        email: str
        age: int
        city: str

        class Settings:
            table_name = "users"

    return User


class TestQueryBuilderCreation:
    """Test QueryBuilder initialization."""

    def test_create_from_find(self, User):
        """Test QueryBuilder created from Table.find()."""
        query = User.find()

        assert isinstance(query, QueryBuilder)
        assert query._model == User
        assert len(query._filters) == 0

    def test_create_with_single_filter(self, User):
        """Test QueryBuilder with single filter."""
        query = User.find(User.age > 25)

        assert isinstance(query, QueryBuilder)
        assert len(query._filters) == 1
        assert isinstance(query._filters[0], SqlExpr)

    def test_create_with_multiple_filters(self, User):
        """Test QueryBuilder with multiple filters."""
        query = User.find(User.age > 25, User.city == "NYC")

        assert len(query._filters) == 2

    def test_create_with_dict_filter(self, User):
        """Test QueryBuilder with dictionary filter."""
        query = User.find({"age": 25})

        assert len(query._filters) == 1
        assert isinstance(query._filters[0], dict)


class TestQueryBuilderFilters:
    """Test filter expressions."""

    def test_filter_equals(self, User):
        """Test equality filter."""
        expr = User.email == "test@example.com"

        assert isinstance(expr, SqlExpr)
        assert expr.column == "email"
        assert expr.op == "="
        assert expr.value == "test@example.com"

    def test_filter_not_equals(self, User):
        """Test not-equal filter."""
        expr = User.name != "Admin"

        assert expr.column == "name"
        assert expr.op == "!="
        assert expr.value == "Admin"

    def test_filter_greater_than(self, User):
        """Test greater-than filter."""
        expr = User.age > 25

        assert expr.column == "age"
        assert expr.op == ">"
        assert expr.value == 25

    def test_filter_greater_equal(self, User):
        """Test greater-than-or-equal filter."""
        expr = User.age >= 18

        assert expr.column == "age"
        assert expr.op == ">="
        assert expr.value == 18

    def test_filter_less_than(self, User):
        """Test less-than filter."""
        expr = User.age < 65

        assert expr.column == "age"
        assert expr.op == "<"
        assert expr.value == 65

    def test_filter_less_equal(self, User):
        """Test less-than-or-equal filter."""
        expr = User.age <= 100

        assert expr.column == "age"
        assert expr.op == "<="
        assert expr.value == 100

    def test_filter_in_list(self, User):
        """Test IN filter."""
        expr = User.city.in_(["NYC", "LA", "SF"])

        assert expr.column == "city"
        assert expr.op == "IN"
        assert expr.value == ["NYC", "LA", "SF"]

    def test_filter_between(self, User):
        """Test BETWEEN filter."""
        expr = User.age.between(18, 65)

        assert expr.column == "age"
        assert expr.op == "BETWEEN"
        assert expr.value == [18, 65]

    def test_filter_like(self, User):
        """Test LIKE filter."""
        expr = User.email.like("%@example.com")

        assert expr.column == "email"
        assert expr.op == "LIKE"
        assert expr.value == "%@example.com"

    def test_filter_ilike(self, User):
        """Test ILIKE (case-insensitive) filter."""
        expr = User.email.ilike("%@EXAMPLE.COM")

        assert expr.column == "email"
        assert expr.op == "ILIKE"
        assert expr.value == "%@EXAMPLE.COM"

    def test_filter_startswith(self, User):
        """Test startswith convenience method."""
        expr = User.name.startswith("A")

        assert expr.column == "name"
        assert expr.op == "LIKE"
        assert expr.value == "A%"

    def test_filter_contains(self, User):
        """Test contains convenience method."""
        expr = User.name.contains("test")

        assert expr.column == "name"
        assert expr.op == "LIKE"
        assert expr.value == "%test%"

    def test_filter_is_null(self, User):
        """Test IS NULL filter."""
        expr = User.city.is_null()

        assert expr.column == "city"
        assert expr.op == "IS NULL"

    def test_filter_is_not_null(self, User):
        """Test IS NOT NULL filter."""
        expr = User.email.is_not_null()

        assert expr.column == "email"
        assert expr.op == "IS NOT NULL"


class TestQueryBuilderOrdering:
    """Test order_by functionality."""

    def test_order_by_ascending(self, User):
        """Test order by ascending."""
        query = User.find().order_by(User.age)

        assert len(query._order_by_spec) == 1
        assert query._order_by_spec[0][0] == "age"
        assert query._order_by_spec[0][1] == "ASC"

    def test_order_by_string(self, User):
        """Test order by using string."""
        query = User.find().order_by("name")

        assert query._order_by_spec[0] == ("name", "ASC")

    def test_order_by_descending_string(self, User):
        """Test order by descending using string with minus."""
        query = User.find().order_by("-age")

        assert query._order_by_spec[0] == ("age", "DESC")

    def test_order_by_multiple_fields(self, User):
        """Test multiple order_by fields."""
        query = User.find().order_by("city", "-age")

        assert len(query._order_by_spec) == 2
        assert query._order_by_spec[0] == ("city", "ASC")
        assert query._order_by_spec[1] == ("age", "DESC")

    def test_order_by_chaining(self, User):
        """Test order_by can be chained."""
        query = User.find().order_by(User.city).order_by("-age")

        assert len(query._order_by_spec) == 2


class TestQueryBuilderPagination:
    """Test limit and offset."""

    def test_limit(self, User):
        """Test limit sets _limit_val."""
        query = User.find().limit(10)

        assert query._limit_val == 10

    def test_offset(self, User):
        """Test offset sets _offset_val."""
        query = User.find().offset(20)

        assert query._offset_val == 20

    def test_limit_and_offset(self, User):
        """Test limit and offset together (pagination)."""
        query = User.find().limit(10).offset(20)

        assert query._limit_val == 10
        assert query._offset_val == 20

    def test_pagination_page_2(self, User):
        """Test pagination calculation for page 2, 20 per page."""
        page = 2
        per_page = 20
        query = User.find().offset((page - 1) * per_page).limit(per_page)

        assert query._offset_val == 20
        assert query._limit_val == 20


class TestQueryBuilderSelect:
    """Test column selection."""

    def test_select_columns(self, User):
        """Test selecting specific columns."""
        query = User.find().select(User.name, User.email)

        assert query._select_cols == ["name", "email"]

    def test_select_with_strings(self, User):
        """Test selecting columns using strings."""
        query = User.find().select("name", "email")

        assert query._select_cols == ["name", "email"]

    def test_select_mixed(self, User):
        """Test selecting with mixed ColumnProxy and strings."""
        query = User.find().select(User.name, "email")

        assert query._select_cols == ["name", "email"]


class TestQueryBuilderChaining:
    """Test query builder method chaining."""

    def test_chained_query(self, User):
        """Test complex chained query."""
        query = (
            User.find(User.age > 25)
            .order_by("-age")
            .offset(10)
            .limit(20)
            .select(User.name, User.email)
        )

        assert len(query._filters) == 1
        assert query._order_by_spec[0] == ("age", "DESC")
        assert query._offset_val == 10
        assert query._limit_val == 20
        assert query._select_cols == ["name", "email"]

    def test_clone_creates_new_instance(self, User):
        """Test _clone creates a new QueryBuilder instance."""
        query1 = User.find()
        query2 = query1.limit(10)

        # Should be different instances
        assert query1 is not query2
        # Original should be unchanged
        assert query1._limit_val == 0
        assert query2._limit_val == 10

    def test_immutable_chaining(self, User):
        """Test that chaining doesn't modify original builder."""
        base_query = User.find(User.age > 18)
        adults = base_query.limit(10)
        # Note: QueryBuilder doesn't have a filter() method, you would use find() again
        # or chain other methods like order_by, limit, offset

        # Base query should be unchanged
        assert base_query._limit_val == 0
        assert adults._limit_val == 10

        # Each chain creates a new instance
        assert base_query is not adults


class TestQueryBuilderSQLGeneration:
    """Test WHERE clause SQL generation."""

    def test_build_where_empty(self, User):
        """Test WHERE clause with no filters."""
        query = User.find()
        where, params = query._build_where_clause()

        assert where == ""
        assert params == []

    def test_build_where_single_filter(self, User):
        """Test WHERE clause with single filter."""
        query = User.find(User.age > 25)
        where, params = query._build_where_clause()

        assert "age" in where
        assert ">" in where
        assert "$1" in where
        assert params == [25]

    def test_build_where_multiple_filters(self, User):
        """Test WHERE clause with multiple filters (AND)."""
        query = User.find(User.age > 25, User.city == "NYC")
        where, params = query._build_where_clause()

        assert "age > $1" in where
        assert "city = $2" in where
        assert "AND" in where
        assert params == [25, "NYC"]

    def test_build_where_dict_filter(self, User):
        """Test WHERE clause with dictionary filter."""
        query = User.find({"age": 25, "city": "NYC"})
        where, params = query._build_where_clause()

        assert "$1" in where
        assert "$2" in where
        assert 25 in params
        assert "NYC" in params

    def test_build_where_in_operator(self, User):
        """Test WHERE clause with IN operator."""
        query = User.find(User.city.in_(["NYC", "LA", "SF"]))
        where, params = query._build_where_clause()

        assert "city" in where
        assert "IN" in where
        assert params == ["NYC", "LA", "SF"]

    def test_build_where_between(self, User):
        """Test WHERE clause with BETWEEN."""
        query = User.find(User.age.between(18, 65))
        where, params = query._build_where_clause()

        assert "age" in where
        assert "BETWEEN" in where
        assert params == [18, 65]

    def test_build_where_is_null(self, User):
        """Test WHERE clause with IS NULL."""
        query = User.find(User.city.is_null())
        where, params = query._build_where_clause()

        assert "city IS NULL" in where
        assert params == []


class TestQueryBuilderExpressionCombining:
    """Test combining expressions with AND/OR."""

    def test_and_operator(self, User):
        """Test combining expressions with & (AND)."""
        expr = (User.age > 18) & (User.age < 65)

        assert isinstance(expr, SqlExpr)
        # The AND combination creates a special SqlExpr
        assert expr.op == "AND"

    def test_or_operator(self, User):
        """Test combining expressions with | (OR)."""
        expr = (User.city == "NYC") | (User.city == "LA")

        assert isinstance(expr, SqlExpr)
        assert expr.op == "OR"
