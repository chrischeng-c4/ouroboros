"""Tests for query extension utilities (SQLAlchemy-style query builders)."""

import pytest
from ouroboros.test import expect
from datetime import datetime, timedelta

from ouroboros.postgres.query_ext import (
    filter_by,
    and_,
    or_,
    not_,
    any_,
    has,
    aliased,
    QueryFragment,
    BooleanClause,
    AliasedClass,
    active_filter,
    date_range_filter,
    in_list_filter,
    null_check_filter,
)
from ouroboros.postgres.columns import SqlExpr


class TestFilterBy:
    """Test filter_by() function."""

    def test_filter_by_single(self):
        """Test single equality filter."""
        filters = filter_by(name="Alice")
        assert len(filters) == 1
        assert isinstance(filters[0], SqlExpr)
        assert filters[0].column == "name"
        assert filters[0].op == "="
        assert filters[0].value == "Alice"

    def test_filter_by_multiple(self):
        """Test multiple equality filters."""
        filters = filter_by(name="Alice", status="active", age=25)
        assert len(filters) == 3

        # Check all filters are SqlExpr
        assert all(isinstance(f, SqlExpr) for f in filters)

        # Check columns (order may vary due to dict)
        columns = {f.column for f in filters}
        assert columns == {"name", "status", "age"}

        # Check all are equality operators
        assert all(f.op == "=" for f in filters)

    def test_filter_by_to_sql(self):
        """Test SQL generation from filter_by results."""
        filters = filter_by(name="Alice")
        sql, params = filters[0].to_sql(1)
        assert sql == "name = $1"
        assert params == ["Alice"]


class TestBooleanClause:
    """Test BooleanClause class."""

    def test_and_clause(self):
        """Test AND clause creation."""
        expr1 = SqlExpr("age", ">", 18)
        expr2 = SqlExpr("status", "=", "active")
        clause = BooleanClause("AND", [expr1, expr2])

        assert clause.operator == "AND"
        assert len(clause.conditions) == 2

        sql, params = clause.to_sql()
        assert "age > $1" in sql
        assert "status = $2" in sql
        assert "AND" in sql
        assert params == [18, "active"]

    def test_or_clause(self):
        """Test OR clause creation."""
        expr1 = SqlExpr("role", "=", "admin")
        expr2 = SqlExpr("role", "=", "moderator")
        clause = BooleanClause("OR", [expr1, expr2])

        assert clause.operator == "OR"
        sql, params = clause.to_sql()
        assert "role = $1" in sql
        assert "role = $2" in sql
        assert "OR" in sql
        assert params == ["admin", "moderator"]

    def test_not_clause(self):
        """Test NOT clause creation."""
        expr = SqlExpr("status", "=", "deleted")
        clause = BooleanClause("NOT", [expr])

        assert clause.operator == "NOT"
        sql, params = clause.to_sql()
        assert "NOT" in sql
        assert "status = $1" in sql
        assert params == ["deleted"]

    def test_nested_clauses(self):
        """Test nested boolean clauses."""
        expr1 = SqlExpr("age", ">", 18)
        expr2 = SqlExpr("role", "=", "admin")
        expr3 = SqlExpr("role", "=", "moderator")

        # (age > 18) AND (role = admin OR role = moderator)
        or_clause = BooleanClause("OR", [expr2, expr3])
        and_clause = BooleanClause("AND", [expr1, or_clause])

        sql, params = and_clause.to_sql()
        assert "age > $1" in sql
        assert "role = $2" in sql or "role = $3" in sql
        assert "AND" in sql
        assert "OR" in sql
        assert params == [18, "admin", "moderator"]

    def test_invalid_operator(self):
        """Test invalid operator raises error."""
        expect(lambda: BooleanClause("XOR", [SqlExpr("a", "=", 1)])).to_raise(ValueError)

    def test_not_requires_one_condition(self):
        """Test NOT requires exactly one condition."""
        expect(lambda: BooleanClause("NOT", [SqlExpr("a", "=", 1), SqlExpr("b", "=", 2)])).to_raise(ValueError)

    def test_and_requires_two_conditions(self):
        """Test AND requires at least two conditions."""
        expect(lambda: BooleanClause("AND", [SqlExpr("a", "=", 1)])).to_raise(ValueError)


class TestCombinators:
    """Test and_(), or_(), not_() combinator functions."""

    def test_and_function(self):
        """Test and_() function."""
        expr1 = SqlExpr("age", ">", 18)
        expr2 = SqlExpr("status", "=", "active")
        clause = and_(expr1, expr2)

        assert isinstance(clause, BooleanClause)
        assert clause.operator == "AND"
        assert len(clause.conditions) == 2

    def test_and_multiple(self):
        """Test and_() with multiple conditions."""
        expr1 = SqlExpr("age", ">", 18)
        expr2 = SqlExpr("status", "=", "active")
        expr3 = SqlExpr("verified", "=", True)
        clause = and_(expr1, expr2, expr3)

        assert len(clause.conditions) == 3
        sql, params = clause.to_sql()
        assert params == [18, "active", True]

    def test_or_function(self):
        """Test or_() function."""
        expr1 = SqlExpr("role", "=", "admin")
        expr2 = SqlExpr("role", "=", "moderator")
        clause = or_(expr1, expr2)

        assert isinstance(clause, BooleanClause)
        assert clause.operator == "OR"

    def test_not_function(self):
        """Test not_() function."""
        expr = SqlExpr("status", "=", "deleted")
        clause = not_(expr)

        assert isinstance(clause, BooleanClause)
        assert clause.operator == "NOT"

    def test_nested_combinators(self):
        """Test nested combinator usage."""
        # (age > 18) AND ((role = admin) OR (role = moderator))
        expr1 = SqlExpr("age", ">", 18)
        expr2 = SqlExpr("role", "=", "admin")
        expr3 = SqlExpr("role", "=", "moderator")

        clause = and_(expr1, or_(expr2, expr3))
        assert clause.operator == "AND"
        assert len(clause.conditions) == 2
        assert isinstance(clause.conditions[1], BooleanClause)
        assert clause.conditions[1].operator == "OR"

    def test_and_requires_two(self):
        """Test and_() requires at least two conditions."""
        expect(lambda: and_(SqlExpr("a", "=", 1))).to_raise(ValueError)

    def test_or_requires_two(self):
        """Test or_() requires at least two conditions."""
        expect(lambda: or_(SqlExpr("a", "=", 1))).to_raise(ValueError)


class TestQueryFragment:
    """Test QueryFragment class."""

    def test_fragment_from_kwargs(self):
        """Test fragment creation from keyword arguments."""
        fragment = QueryFragment(status="active", age=25)
        conditions = fragment.to_conditions()

        assert len(conditions) == 2
        assert all(isinstance(c, SqlExpr) for c in conditions)

    def test_fragment_from_expressions(self):
        """Test fragment creation from SqlExpr objects."""
        expr1 = SqlExpr("age", ">", 18)
        expr2 = SqlExpr("status", "=", "active")
        fragment = QueryFragment(expr1, expr2)

        conditions = fragment.to_conditions()
        assert len(conditions) == 2
        assert conditions[0] is expr1
        assert conditions[1] is expr2

    def test_fragment_mixed(self):
        """Test fragment with both expressions and kwargs."""
        expr = SqlExpr("age", ">", 18)
        fragment = QueryFragment(expr, status="active")

        conditions = fragment.to_conditions()
        assert len(conditions) == 2

    def test_fragment_django_style_lookups(self):
        """Test Django-style field lookups."""
        fragment = QueryFragment(age__gt=18, age__lte=65)
        conditions = fragment.to_conditions()

        assert len(conditions) == 2
        # Check operators
        ops = {c.op for c in conditions}
        assert ">" in ops
        assert "<=" in ops

    def test_fragment_isnull_lookup(self):
        """Test isnull lookup."""
        fragment1 = QueryFragment(deleted_at__isnull=True)
        fragment2 = QueryFragment(deleted_at__isnull=False)

        cond1 = fragment1.to_conditions()[0]
        cond2 = fragment2.to_conditions()[0]

        assert cond1.op == "IS NULL"
        assert cond2.op == "IS NOT NULL"

    def test_fragment_and_combination(self):
        """Test combining fragments with &."""
        frag1 = QueryFragment(status="active")
        frag2 = QueryFragment(age__gt=18)
        combined = frag1 & frag2

        assert isinstance(combined, QueryFragment)
        conditions = combined.to_conditions()
        assert len(conditions) == 2

    def test_fragment_or_combination(self):
        """Test combining fragments with |."""
        frag1 = QueryFragment(role="admin")
        frag2 = QueryFragment(role="moderator")
        combined = frag1 | frag2

        assert isinstance(combined, BooleanClause)
        assert combined.operator == "OR"

    def test_fragment_not(self):
        """Test negating fragment with ~."""
        fragment = QueryFragment(status="deleted")
        negated = ~fragment

        assert isinstance(negated, BooleanClause)
        assert negated.operator == "NOT"

    def test_fragment_iteration(self):
        """Test fragment can be unpacked."""
        fragment = QueryFragment(status="active", age=25)
        conditions = list(fragment)
        assert len(conditions) == 2

    def test_invalid_lookup(self):
        """Test invalid lookup raises error."""
        expect(lambda: QueryFragment(age__invalid=18)).to_raise(ValueError)


class TestConvenienceFilters:
    """Test convenience filter functions."""

    def test_active_filter(self):
        """Test active_filter()."""
        fragment = active_filter()
        conditions = fragment.to_conditions()

        assert len(conditions) == 1
        assert conditions[0].column == "status"
        assert conditions[0].value == "active"

    def test_active_filter_custom(self):
        """Test active_filter() with custom column and value."""
        fragment = active_filter(column="state", value="enabled")
        conditions = fragment.to_conditions()

        assert conditions[0].column == "state"
        assert conditions[0].value == "enabled"

    def test_date_range_filter_both(self):
        """Test date_range_filter() with both start and end."""
        now = datetime.now()
        week_ago = now - timedelta(days=7)

        fragment = date_range_filter("created_at", week_ago, now)
        conditions = fragment.to_conditions()

        assert len(conditions) == 2
        ops = {c.op for c in conditions}
        assert ">=" in ops
        assert "<=" in ops

    def test_date_range_filter_start_only(self):
        """Test date_range_filter() with start only."""
        week_ago = datetime.now() - timedelta(days=7)
        fragment = date_range_filter("created_at", start=week_ago)
        conditions = fragment.to_conditions()

        assert len(conditions) == 1
        assert conditions[0].op == ">="

    def test_date_range_filter_end_only(self):
        """Test date_range_filter() with end only."""
        now = datetime.now()
        fragment = date_range_filter("created_at", end=now)
        conditions = fragment.to_conditions()

        assert len(conditions) == 1
        assert conditions[0].op == "<="

    def test_in_list_filter(self):
        """Test in_list_filter()."""
        fragment = in_list_filter("role", ["admin", "moderator"])
        conditions = fragment.to_conditions()

        assert len(conditions) == 1
        assert conditions[0].op == "IN"
        assert conditions[0].value == ["admin", "moderator"]

    def test_null_check_filter_is_null(self):
        """Test null_check_filter() for IS NULL."""
        fragment = null_check_filter("deleted_at", is_null=True)
        conditions = fragment.to_conditions()

        assert len(conditions) == 1
        assert conditions[0].op == "IS NULL"

    def test_null_check_filter_is_not_null(self):
        """Test null_check_filter() for IS NOT NULL."""
        fragment = null_check_filter("deleted_at", is_null=False)
        conditions = fragment.to_conditions()

        assert len(conditions) == 1
        assert conditions[0].op == "IS NOT NULL"


class TestAliased:
    """Test aliased() function and AliasedClass."""

    def test_aliased_creation(self):
        """Test creating an aliased class."""
        # Mock Table class
        class User:
            __name__ = "User"

        Manager = aliased(User)
        assert isinstance(Manager, AliasedClass)
        assert Manager._table_class is User
        assert Manager._alias == "user_1"

    def test_aliased_custom_name(self):
        """Test aliased() with custom alias name."""
        class User:
            __name__ = "User"

        Manager = aliased(User, "mgr")
        assert Manager._alias == "mgr"

    def test_aliased_repr(self):
        """Test AliasedClass repr."""
        class User:
            __name__ = "User"

        Manager = aliased(User, "mgr")
        repr_str = repr(Manager)
        assert "User" in repr_str
        assert "mgr" in repr_str


class TestRelationshipFilters:
    """Test any_() and has() functions."""

    def test_any_not_implemented(self):
        """Test any_() raises NotImplementedError."""
        # These are placeholders for future implementation
        expect(lambda: any_(None, SqlExpr("views", ">", 1000))).to_raise(NotImplementedError)

    def test_has_not_implemented(self):
        """Test has() raises NotImplementedError."""
        expect(lambda: has(None, SqlExpr("verified", "=", True))).to_raise(NotImplementedError)


class TestBooleanClauseOperators:
    """Test BooleanClause operator overloading."""

    def test_clause_and_operator(self):
        """Test & operator on BooleanClause."""
        clause1 = and_(SqlExpr("age", ">", 18), SqlExpr("status", "=", "active"))
        clause2 = SqlExpr("verified", "=", True)
        combined = clause1 & clause2

        assert isinstance(combined, BooleanClause)
        assert combined.operator == "AND"

    def test_clause_or_operator(self):
        """Test | operator on BooleanClause."""
        clause1 = and_(SqlExpr("age", ">", 18), SqlExpr("status", "=", "active"))
        clause2 = SqlExpr("role", "=", "admin")
        combined = clause1 | clause2

        assert isinstance(combined, BooleanClause)
        assert combined.operator == "OR"

    def test_clause_invert_operator(self):
        """Test ~ operator on BooleanClause."""
        clause = and_(SqlExpr("age", ">", 18), SqlExpr("status", "=", "active"))
        negated = ~clause

        assert isinstance(negated, BooleanClause)
        assert negated.operator == "NOT"


class TestComplexScenarios:
    """Test complex real-world query scenarios."""

    def test_complex_user_query(self):
        """Test complex user filtering scenario."""
        # Find active verified users over 18, who are either admin or moderator
        active = QueryFragment(status="active", verified=True)
        adult = SqlExpr("age", ">", 18)
        privileged = or_(SqlExpr("role", "=", "admin"), SqlExpr("role", "=", "moderator"))

        # Combine all conditions
        conditions = [*active, adult, privileged]
        # active has 2 conditions, adult is 1, privileged is 1 = 4 total
        assert len(conditions) == 4

    def test_complex_date_and_status_query(self):
        """Test complex date range and status query."""
        # Find posts created in the last 7 days that are either published or featured
        now = datetime.now()
        week_ago = now - timedelta(days=7)

        recent = date_range_filter("created_at", week_ago, now)
        status_check = or_(
            SqlExpr("status", "=", "published"),
            SqlExpr("status", "=", "featured")
        )

        conditions = [*recent, status_check]
        assert len(conditions) == 3  # 2 from date_range + 1 from or_

    def test_reusable_fragments(self):
        """Test reusing fragments across queries."""
        # Define reusable fragments
        active = active_filter()
        verified = QueryFragment(verified=True)
        premium = QueryFragment(subscription_tier__in=["pro", "enterprise"])

        # Combine in different ways
        combo1 = active & verified
        combo2 = active & verified & premium

        assert len(combo1.to_conditions()) == 2
        assert len(combo2.to_conditions()) == 3

    def test_fragment_with_not(self):
        """Test fragment with negation."""
        deleted = QueryFragment(status="deleted")
        not_deleted = ~deleted

        assert isinstance(not_deleted, BooleanClause)
        assert not_deleted.operator == "NOT"

    def test_multiple_or_conditions(self):
        """Test multiple OR conditions."""
        # Find users with any of multiple roles
        roles = ["admin", "moderator", "editor", "contributor"]
        role_conditions = [SqlExpr("role", "=", role) for role in roles]

        # Chain them with OR
        combined = role_conditions[0]
        for cond in role_conditions[1:]:
            combined = combined | cond

        # Should create nested OR clauses
        assert isinstance(combined, SqlExpr) or isinstance(combined, BooleanClause)


class MockTable:
    """Mock Table class for testing."""

    _table_name = "test_table"
    _columns = {"id": int, "name": str, "status": str, "age": int, "verified": bool}

    @classmethod
    def __table_name__(cls):
        return cls._table_name

    def __init__(self, **kwargs):
        self.id = kwargs.get("id")
        self.name = kwargs.get("name")
        self.status = kwargs.get("status")
        self.age = kwargs.get("age")
        self.verified = kwargs.get("verified")


class TestQueryBuilderIntegration:
    """Test integration between query_ext and QueryBuilder."""

    def test_query_builder_with_filter_by(self):
        """Test QueryBuilder accepts filter_by results."""
        from ouroboros.postgres.query import QueryBuilder

        filters = filter_by(name="Alice", status="active")
        qb = QueryBuilder(MockTable, tuple(filters))

        where_clause, params = qb._build_where_clause()
        assert "name = $1" in where_clause or "name = $2" in where_clause
        assert "status = $1" in where_clause or "status = $2" in where_clause
        assert "AND" in where_clause
        assert set(params) == {"Alice", "active"}

    def test_query_builder_with_and_clause(self):
        """Test QueryBuilder accepts and_() results."""
        from ouroboros.postgres.query import QueryBuilder

        condition = and_(
            SqlExpr("age", ">", 18),
            SqlExpr("status", "=", "active")
        )
        qb = QueryBuilder(MockTable, (condition,))

        where_clause, params = qb._build_where_clause()
        assert "age > $1" in where_clause
        assert "status = $2" in where_clause
        assert "AND" in where_clause
        assert params == [18, "active"]

    def test_query_builder_with_or_clause(self):
        """Test QueryBuilder accepts or_() results."""
        from ouroboros.postgres.query import QueryBuilder

        condition = or_(
            SqlExpr("role", "=", "admin"),
            SqlExpr("role", "=", "moderator")
        )
        qb = QueryBuilder(MockTable, (condition,))

        where_clause, params = qb._build_where_clause()
        assert "role = $1" in where_clause
        assert "role = $2" in where_clause
        assert "OR" in where_clause
        assert params == ["admin", "moderator"]

    def test_query_builder_with_not_clause(self):
        """Test QueryBuilder accepts not_() results."""
        from ouroboros.postgres.query import QueryBuilder

        condition = not_(SqlExpr("status", "=", "deleted"))
        qb = QueryBuilder(MockTable, (condition,))

        where_clause, params = qb._build_where_clause()
        assert "NOT" in where_clause
        assert "status = $1" in where_clause
        assert params == ["deleted"]

    def test_query_builder_with_nested_clauses(self):
        """Test QueryBuilder with nested boolean clauses."""
        from ouroboros.postgres.query import QueryBuilder

        # (age > 18) AND ((role = admin) OR (role = moderator))
        condition = and_(
            SqlExpr("age", ">", 18),
            or_(SqlExpr("role", "=", "admin"), SqlExpr("role", "=", "moderator"))
        )
        qb = QueryBuilder(MockTable, (condition,))

        where_clause, params = qb._build_where_clause()
        assert "age > $1" in where_clause
        assert "AND" in where_clause
        assert "OR" in where_clause
        assert params == [18, "admin", "moderator"]

    def test_query_builder_with_query_fragment(self):
        """Test QueryBuilder with QueryFragment."""
        from ouroboros.postgres.query import QueryBuilder

        fragment = QueryFragment(status="active", verified=True)
        qb = QueryBuilder(MockTable, tuple(fragment))

        where_clause, params = qb._build_where_clause()
        assert ("status = $1" in where_clause) or ("status = $2" in where_clause)
        assert ("verified = $1" in where_clause) or ("verified = $2" in where_clause)
        assert "AND" in where_clause

    def test_query_builder_mixed_filters(self):
        """Test QueryBuilder with mixed filter types."""
        from ouroboros.postgres.query import QueryBuilder

        fragment = QueryFragment(status="active")
        condition = or_(
            SqlExpr("role", "=", "admin"),
            SqlExpr("role", "=", "moderator")
        )

        qb = QueryBuilder(MockTable, (*fragment, condition))

        where_clause, params = qb._build_where_clause()
        # Should have status = active AND (role = admin OR role = moderator)
        assert "status = $1" in where_clause
        assert "AND" in where_clause
        assert "OR" in where_clause

    def test_query_builder_complex_scenario(self):
        """Test complex real-world scenario."""
        from ouroboros.postgres.query import QueryBuilder

        # Find active users over 18 who are admins or verified moderators
        active = QueryFragment(status="active")
        adult = SqlExpr("age", ">", 18)
        privileged = or_(
            SqlExpr("role", "=", "admin"),
            and_(
                SqlExpr("role", "=", "moderator"),
                SqlExpr("verified", "=", True)
            )
        )

        qb = QueryBuilder(MockTable, (*active, adult, privileged))
        where_clause, params = qb._build_where_clause()

        # Verify all conditions are present
        assert "status = $1" in where_clause
        assert "age > $2" in where_clause
        assert "role" in where_clause
        assert "verified" in where_clause
        assert "AND" in where_clause
        assert "OR" in where_clause

    def test_query_builder_with_dict_filters(self):
        """Test QueryBuilder with dict filters (backward compatibility)."""
        from ouroboros.postgres.query import QueryBuilder

        qb = QueryBuilder(MockTable, ({"status": "active", "verified": True},))

        where_clause, params = qb._build_where_clause()
        assert "status = $1" in where_clause or "status = $2" in where_clause
        assert "verified = $1" in where_clause or "verified = $2" in where_clause
        assert "AND" in where_clause

    def test_query_builder_filter_and_boolean_clause(self):
        """Test mixing regular SqlExpr and BooleanClause."""
        from ouroboros.postgres.query import QueryBuilder

        regular = SqlExpr("status", "=", "active")
        boolean = or_(
            SqlExpr("role", "=", "admin"),
            SqlExpr("role", "=", "moderator")
        )

        qb = QueryBuilder(MockTable, (regular, boolean))
        where_clause, params = qb._build_where_clause()

        assert "status = $1" in where_clause
        assert "role" in where_clause
        assert "AND" in where_clause  # Top-level AND between filters
        assert "OR" in where_clause   # OR inside the BooleanClause
        assert params == ["active", "admin", "moderator"]


class TestSQLGeneration:
    """Test SQL generation with various query_ext constructs."""

    def test_and_sql_generation(self):
        """Test SQL generation for AND clause."""
        clause = and_(
            SqlExpr("age", ">", 18),
            SqlExpr("status", "=", "active")
        )
        sql, params = clause.to_sql()
        assert sql == "age > $1 AND status = $2"
        assert params == [18, "active"]

    def test_or_sql_generation(self):
        """Test SQL generation for OR clause."""
        clause = or_(
            SqlExpr("role", "=", "admin"),
            SqlExpr("role", "=", "moderator")
        )
        sql, params = clause.to_sql()
        assert sql == "role = $1 OR role = $2"
        assert params == ["admin", "moderator"]

    def test_not_sql_generation(self):
        """Test SQL generation for NOT clause."""
        clause = not_(SqlExpr("status", "=", "deleted"))
        sql, params = clause.to_sql()
        assert sql == "NOT (status = $1)"
        assert params == ["deleted"]

    def test_nested_and_or_sql(self):
        """Test SQL generation for nested AND/OR."""
        clause = and_(
            SqlExpr("age", ">", 18),
            or_(
                SqlExpr("role", "=", "admin"),
                SqlExpr("role", "=", "moderator")
            )
        )
        sql, params = clause.to_sql()
        assert "age > $1" in sql
        assert "AND" in sql
        assert "(role = $2 OR role = $3)" in sql
        assert params == [18, "admin", "moderator"]

    def test_double_negation_sql(self):
        """Test SQL generation for NOT NOT."""
        clause = not_(not_(SqlExpr("status", "=", "active")))
        sql, params = clause.to_sql()
        assert sql == "NOT (NOT (status = $1))"
        assert params == ["active"]

    def test_complex_nested_sql(self):
        """Test complex nested boolean logic."""
        # ((a AND b) OR (c AND d)) AND NOT e
        clause = and_(
            or_(
                and_(SqlExpr("a", "=", 1), SqlExpr("b", "=", 2)),
                and_(SqlExpr("c", "=", 3), SqlExpr("d", "=", 4))
            ),
            not_(SqlExpr("e", "=", 5))
        )
        sql, params = clause.to_sql()

        # Check structure
        assert "AND" in sql
        assert "OR" in sql
        assert "NOT" in sql
        assert params == [1, 2, 3, 4, 5]

    def test_parameter_indexing(self):
        """Test that parameter indices are correct."""
        clause = and_(
            SqlExpr("a", "=", "value_a"),
            SqlExpr("b", "=", "value_b"),
            SqlExpr("c", "=", "value_c")
        )
        sql, params = clause.to_sql(1)

        assert "$1" in sql
        assert "$2" in sql
        assert "$3" in sql
        assert params == ["value_a", "value_b", "value_c"]

    def test_parameter_indexing_with_offset(self):
        """Test parameter indexing with custom starting index."""
        clause = and_(
            SqlExpr("a", "=", "value_a"),
            SqlExpr("b", "=", "value_b")
        )
        sql, params = clause.to_sql(5)

        assert "$5" in sql
        assert "$6" in sql
        assert params == ["value_a", "value_b"]
