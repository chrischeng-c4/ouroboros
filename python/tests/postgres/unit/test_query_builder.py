"""
Unit tests for QueryBuilder class.

Tests the query builder Python API without requiring a real database connection.
"""
import pytest
from ouroboros.postgres import Table, Column
from ouroboros.postgres.query import QueryBuilder
from ouroboros.postgres.columns import SqlExpr, ColumnProxy
from ouroboros.test import expect


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

        expect(isinstance(query, QueryBuilder)).to_be_true()
        expect(query._model).to_equal(User)
        expect(len(query._filters)).to_equal(0)

    def test_create_with_single_filter(self, User):
        """Test QueryBuilder with single filter."""
        query = User.find(User.age > 25)

        expect(isinstance(query, QueryBuilder)).to_be_true()
        expect(len(query._filters)).to_equal(1)
        expect(isinstance(query._filters[0], SqlExpr)).to_be_true()

    def test_create_with_multiple_filters(self, User):
        """Test QueryBuilder with multiple filters."""
        query = User.find(User.age > 25, User.city == "NYC")

        expect(len(query._filters)).to_equal(2)

    def test_create_with_dict_filter(self, User):
        """Test QueryBuilder with dictionary filter."""
        query = User.find({"age": 25})

        expect(len(query._filters)).to_equal(1)
        expect(isinstance(query._filters[0], dict)).to_be_true()


class TestQueryBuilderFilters:
    """Test filter expressions."""

    def test_filter_equals(self, User):
        """Test equality filter."""
        expr = User.email == "test@example.com"

        expect(isinstance(expr, SqlExpr)).to_be_true()
        expect(expr.column).to_equal("email")
        expect(expr.op).to_equal("=")
        expect(expr.value).to_equal("test@example.com")

    def test_filter_not_equals(self, User):
        """Test not-equal filter."""
        expr = User.name != "Admin"

        expect(expr.column).to_equal("name")
        expect(expr.op).to_equal("!=")
        expect(expr.value).to_equal("Admin")

    def test_filter_greater_than(self, User):
        """Test greater-than filter."""
        expr = User.age > 25

        expect(expr.column).to_equal("age")
        expect(expr.op).to_equal(">")
        expect(expr.value).to_equal(25)

    def test_filter_greater_equal(self, User):
        """Test greater-than-or-equal filter."""
        expr = User.age >= 18

        expect(expr.column).to_equal("age")
        expect(expr.op).to_equal(">=")
        expect(expr.value).to_equal(18)

    def test_filter_less_than(self, User):
        """Test less-than filter."""
        expr = User.age < 65

        expect(expr.column).to_equal("age")
        expect(expr.op).to_equal("<")
        expect(expr.value).to_equal(65)

    def test_filter_less_equal(self, User):
        """Test less-than-or-equal filter."""
        expr = User.age <= 100

        expect(expr.column).to_equal("age")
        expect(expr.op).to_equal("<=")
        expect(expr.value).to_equal(100)

    def test_filter_in_list(self, User):
        """Test IN filter."""
        expr = User.city.in_(["NYC", "LA", "SF"])

        expect(expr.column).to_equal("city")
        expect(expr.op).to_equal("IN")
        expect(expr.value).to_equal(["NYC", "LA", "SF"])

    def test_filter_between(self, User):
        """Test BETWEEN filter."""
        expr = User.age.between(18, 65)

        expect(expr.column).to_equal("age")
        expect(expr.op).to_equal("BETWEEN")
        expect(expr.value).to_equal([18, 65])

    def test_filter_like(self, User):
        """Test LIKE filter."""
        expr = User.email.like("%@example.com")

        expect(expr.column).to_equal("email")
        expect(expr.op).to_equal("LIKE")
        expect(expr.value).to_equal("%@example.com")

    def test_filter_ilike(self, User):
        """Test ILIKE (case-insensitive) filter."""
        expr = User.email.ilike("%@EXAMPLE.COM")

        expect(expr.column).to_equal("email")
        expect(expr.op).to_equal("ILIKE")
        expect(expr.value).to_equal("%@EXAMPLE.COM")

    def test_filter_startswith(self, User):
        """Test startswith convenience method."""
        expr = User.name.startswith("A")

        expect(expr.column).to_equal("name")
        expect(expr.op).to_equal("LIKE")
        expect(expr.value).to_equal("A%")

    def test_filter_contains(self, User):
        """Test contains convenience method."""
        expr = User.name.contains("test")

        expect(expr.column).to_equal("name")
        expect(expr.op).to_equal("LIKE")
        expect(expr.value).to_equal("%test%")

    def test_filter_is_null(self, User):
        """Test IS NULL filter."""
        expr = User.city.is_null()

        expect(expr.column).to_equal("city")
        expect(expr.op).to_equal("IS NULL")

    def test_filter_is_not_null(self, User):
        """Test IS NOT NULL filter."""
        expr = User.email.is_not_null()

        expect(expr.column).to_equal("email")
        expect(expr.op).to_equal("IS NOT NULL")


class TestQueryBuilderOrdering:
    """Test order_by functionality."""

    def test_order_by_ascending(self, User):
        """Test order by ascending."""
        query = User.find().order_by(User.age)

        expect(len(query._order_by_spec)).to_equal(1)
        expect(query._order_by_spec[0][0]).to_equal("age")
        expect(query._order_by_spec[0][1]).to_equal("ASC")

    def test_order_by_string(self, User):
        """Test order by using string."""
        query = User.find().order_by("name")

        expect(query._order_by_spec[0]).to_equal(("name", "ASC"))

    def test_order_by_descending_string(self, User):
        """Test order by descending using string with minus."""
        query = User.find().order_by("-age")

        expect(query._order_by_spec[0]).to_equal(("age", "DESC"))

    def test_order_by_multiple_fields(self, User):
        """Test multiple order_by fields."""
        query = User.find().order_by("city", "-age")

        expect(len(query._order_by_spec)).to_equal(2)
        expect(query._order_by_spec[0]).to_equal(("city", "ASC"))
        expect(query._order_by_spec[1]).to_equal(("age", "DESC"))

    def test_order_by_chaining(self, User):
        """Test order_by can be chained."""
        query = User.find().order_by(User.city).order_by("-age")

        expect(len(query._order_by_spec)).to_equal(2)


class TestQueryBuilderPagination:
    """Test limit and offset."""

    def test_limit(self, User):
        """Test limit sets _limit_val."""
        query = User.find().limit(10)

        expect(query._limit_val).to_equal(10)

    def test_offset(self, User):
        """Test offset sets _offset_val."""
        query = User.find().offset(20)

        expect(query._offset_val).to_equal(20)

    def test_limit_and_offset(self, User):
        """Test limit and offset together (pagination)."""
        query = User.find().limit(10).offset(20)

        expect(query._limit_val).to_equal(10)
        expect(query._offset_val).to_equal(20)

    def test_pagination_page_2(self, User):
        """Test pagination calculation for page 2, 20 per page."""
        page = 2
        per_page = 20
        query = User.find().offset((page - 1) * per_page).limit(per_page)

        expect(query._offset_val).to_equal(20)
        expect(query._limit_val).to_equal(20)


class TestQueryBuilderSelect:
    """Test column selection."""

    def test_select_columns(self, User):
        """Test selecting specific columns."""
        query = User.find().select(User.name, User.email)

        expect(query._select_cols).to_equal(["name", "email"])

    def test_select_with_strings(self, User):
        """Test selecting columns using strings."""
        query = User.find().select("name", "email")

        expect(query._select_cols).to_equal(["name", "email"])

    def test_select_mixed(self, User):
        """Test selecting with mixed ColumnProxy and strings."""
        query = User.find().select(User.name, "email")

        expect(query._select_cols).to_equal(["name", "email"])


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

        expect(len(query._filters)).to_equal(1)
        expect(query._order_by_spec[0]).to_equal(("age", "DESC"))
        expect(query._offset_val).to_equal(10)
        expect(query._limit_val).to_equal(20)
        expect(query._select_cols).to_equal(["name", "email"])

    def test_clone_creates_new_instance(self, User):
        """Test _clone creates a new QueryBuilder instance."""
        query1 = User.find()
        query2 = query1.limit(10)

        # Should be different instances
        expect(query1 is not query2).to_be_true()
        # Original should be unchanged
        expect(query1._limit_val).to_equal(0)
        expect(query2._limit_val).to_equal(10)

    def test_immutable_chaining(self, User):
        """Test that chaining doesn't modify original builder."""
        base_query = User.find(User.age > 18)
        adults = base_query.limit(10)
        # Note: QueryBuilder doesn't have a filter() method, you would use find() again
        # or chain other methods like order_by, limit, offset

        # Base query should be unchanged
        expect(base_query._limit_val).to_equal(0)
        expect(adults._limit_val).to_equal(10)

        # Each chain creates a new instance
        expect(base_query is not adults).to_be_true()


class TestQueryBuilderSQLGeneration:
    """Test WHERE clause SQL generation."""

    def test_build_where_empty(self, User):
        """Test WHERE clause with no filters."""
        query = User.find()
        where, params = query._build_where_clause()

        expect(where).to_equal("")
        expect(params).to_equal([])

    def test_build_where_single_filter(self, User):
        """Test WHERE clause with single filter."""
        query = User.find(User.age > 25)
        where, params = query._build_where_clause()

        expect("age" in where).to_be_true()
        expect(">" in where).to_be_true()
        expect("$1" in where).to_be_true()
        expect(params).to_equal([25])

    def test_build_where_multiple_filters(self, User):
        """Test WHERE clause with multiple filters (AND)."""
        query = User.find(User.age > 25, User.city == "NYC")
        where, params = query._build_where_clause()

        expect("age > $1" in where).to_be_true()
        expect("city = $2" in where).to_be_true()
        expect("AND" in where).to_be_true()
        expect(params).to_equal([25, "NYC"])

    def test_build_where_dict_filter(self, User):
        """Test WHERE clause with dictionary filter."""
        query = User.find({"age": 25, "city": "NYC"})
        where, params = query._build_where_clause()

        expect("$1" in where).to_be_true()
        expect("$2" in where).to_be_true()
        expect(25 in params).to_be_true()
        expect("NYC" in params).to_be_true()

    def test_build_where_in_operator(self, User):
        """Test WHERE clause with IN operator."""
        query = User.find(User.city.in_(["NYC", "LA", "SF"]))
        where, params = query._build_where_clause()

        expect("city" in where).to_be_true()
        expect("IN" in where).to_be_true()
        expect(params).to_equal(["NYC", "LA", "SF"])

    def test_build_where_between(self, User):
        """Test WHERE clause with BETWEEN."""
        query = User.find(User.age.between(18, 65))
        where, params = query._build_where_clause()

        expect("age" in where).to_be_true()
        expect("BETWEEN" in where).to_be_true()
        expect(params).to_equal([18, 65])

    def test_build_where_is_null(self, User):
        """Test WHERE clause with IS NULL."""
        query = User.find(User.city.is_null())
        where, params = query._build_where_clause()

        expect("city IS NULL" in where).to_be_true()
        expect(params).to_equal([])


class TestQueryBuilderExpressionCombining:
    """Test combining expressions with AND/OR."""

    def test_and_operator(self, User):
        """Test combining expressions with & (AND)."""
        expr = (User.age > 18) & (User.age < 65)

        expect(isinstance(expr, SqlExpr)).to_be_true()
        # The AND combination creates a special SqlExpr
        expect(expr.op).to_equal("AND")

    def test_or_operator(self, User):
        """Test combining expressions with | (OR)."""
        expr = (User.city == "NYC") | (User.city == "LA")

        expect(isinstance(expr, SqlExpr)).to_be_true()
        expect(expr.op).to_equal("OR")
