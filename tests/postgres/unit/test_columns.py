"""
Unit tests for Column descriptors and SQL expressions.

Tests ColumnProxy, SqlExpr, and Column classes.
"""
import pytest
from data_bridge.test import expect
from data_bridge.postgres import Table, Column
from data_bridge.postgres.columns import ColumnProxy, SqlExpr


class TestColumnProxy:
    """Test ColumnProxy descriptor behavior."""

    def test_column_proxy_creation(self):
        """Test ColumnProxy can be created."""
        proxy = ColumnProxy("email")

        expect(proxy.name).to_equal("email")
        expect(proxy.model).to_be_none()

    def test_column_proxy_with_model(self):
        """Test ColumnProxy with model reference."""

        class User(Table):
            email: str

        proxy = User.email

        expect(isinstance(proxy, ColumnProxy)).to_be_true()
        expect(proxy.name).to_equal("email")
        expect(proxy.model).to_equal(User)

    def test_column_proxy_repr(self):
        """Test ColumnProxy __repr__."""
        proxy = ColumnProxy("email")
        repr_str = repr(proxy)

        expect("ColumnProxy" in repr_str).to_be_true()
        expect("email" in repr_str).to_be_true()

    def test_column_proxy_hashable(self):
        """Test ColumnProxy is hashable (can be dict key)."""

        class User(Table):
            email: str

        proxy = User.email

        # Should be usable as dict key
        d = {proxy: "value"}
        expect(d[proxy]).to_equal("value")

    def test_descriptor_class_access(self):
        """Test descriptor returns self on class access."""

        class User(Table):
            email: str

        # Class access should return ColumnProxy
        expect(isinstance(User.email, ColumnProxy)).to_be_true()

    def test_descriptor_instance_access(self):
        """Test descriptor returns value on instance access."""

        class User(Table):
            email: str

        user = User(email="test@example.com")

        # Instance access should return value
        expect(user.email).to_equal("test@example.com")

    def test_descriptor_set(self):
        """Test descriptor __set__ updates _data."""

        class User(Table):
            email: str

        user = User(email="old@example.com")
        user.email = "new@example.com"

        expect(user._data["email"]).to_equal("new@example.com")


class TestSqlExprCreation:
    """Test SqlExpr creation from operators."""

    def test_equals(self):
        """Test == creates SqlExpr."""

        class User(Table):
            age: int

        expr = User.age == 25

        expect(isinstance(expr, SqlExpr)).to_be_true()
        expect(expr.column).to_equal("age")
        expect(expr.op).to_equal("=")
        expect(expr.value).to_equal(25)

    def test_not_equals(self):
        """Test != creates SqlExpr."""

        class User(Table):
            status: str

        expr = User.status != "deleted"

        expect(expr.op).to_equal("!=")
        expect(expr.value).to_equal("deleted")

    def test_greater_than(self):
        """Test > creates SqlExpr."""

        class User(Table):
            age: int

        expr = User.age > 18

        expect(expr.op).to_equal(">")
        expect(expr.value).to_equal(18)

    def test_greater_equal(self):
        """Test >= creates SqlExpr."""

        class User(Table):
            score: int

        expr = User.score >= 100

        expect(expr.op).to_equal(">=")

    def test_less_than(self):
        """Test < creates SqlExpr."""

        class User(Table):
            age: int

        expr = User.age < 65

        expect(expr.op).to_equal("<")

    def test_less_equal(self):
        """Test <= creates SqlExpr."""

        class User(Table):
            age: int

        expr = User.age <= 100

        expect(expr.op).to_equal("<=")


class TestSqlExprMethods:
    """Test SqlExpr methods (in_, like, etc.)."""

    def test_in_method(self):
        """Test in_() method."""

        class User(Table):
            city: str

        expr = User.city.in_(["NYC", "LA", "SF"])

        expect(expr.op).to_equal("IN")
        expect(expr.value).to_equal(["NYC", "LA", "SF"])

    def test_between_method(self):
        """Test between() method."""

        class User(Table):
            age: int

        expr = User.age.between(18, 65)

        expect(expr.op).to_equal("BETWEEN")
        expect(expr.value).to_equal([18, 65])

    def test_is_null_method(self):
        """Test is_null() method."""

        class User(Table):
            middle_name: str

        expr = User.middle_name.is_null()

        expect(expr.op).to_equal("IS NULL")
        expect(expr.value).to_be_none()

    def test_is_not_null_method(self):
        """Test is_not_null() method."""

        class User(Table):
            email: str

        expr = User.email.is_not_null()

        expect(expr.op).to_equal("IS NOT NULL")

    def test_like_method(self):
        """Test like() method."""

        class User(Table):
            email: str

        expr = User.email.like("%@example.com")

        expect(expr.op).to_equal("LIKE")
        expect(expr.value).to_equal("%@example.com")

    def test_ilike_method(self):
        """Test ilike() case-insensitive method."""

        class User(Table):
            email: str

        expr = User.email.ilike("%@EXAMPLE.COM")

        expect(expr.op).to_equal("ILIKE")
        expect(expr.value).to_equal("%@EXAMPLE.COM")

    def test_startswith_method(self):
        """Test startswith() convenience method."""

        class User(Table):
            name: str

        expr = User.name.startswith("A")

        expect(expr.op).to_equal("LIKE")
        expect(expr.value).to_equal("A%")

    def test_contains_method(self):
        """Test contains() convenience method."""

        class User(Table):
            bio: str

        expr = User.bio.contains("python")

        expect(expr.op).to_equal("LIKE")
        expect(expr.value).to_equal("%python%")


class TestSqlExprToSQL:
    """Test SqlExpr to_sql() conversion."""

    def test_to_sql_equals(self):
        """Test to_sql for equals operator."""
        expr = SqlExpr("age", "=", 25)
        sql, params = expr.to_sql()

        expect(sql).to_equal("age = $1")
        expect(params).to_equal([25])

    def test_to_sql_greater_than(self):
        """Test to_sql for greater than."""
        expr = SqlExpr("age", ">", 18)
        sql, params = expr.to_sql()

        expect(sql).to_equal("age > $1")
        expect(params).to_equal([18])

    def test_to_sql_in(self):
        """Test to_sql for IN operator."""
        expr = SqlExpr("city", "IN", ["NYC", "LA", "SF"])
        sql, params = expr.to_sql()

        expect("city IN" in sql).to_be_true()
        expect("$1" in sql).to_be_true()
        expect("$2" in sql).to_be_true()
        expect("$3" in sql).to_be_true()
        expect(params).to_equal(["NYC", "LA", "SF"])

    def test_to_sql_between(self):
        """Test to_sql for BETWEEN."""
        expr = SqlExpr("age", "BETWEEN", [18, 65])
        sql, params = expr.to_sql()

        expect("age BETWEEN $1 AND $2" in sql).to_be_true()
        expect(params).to_equal([18, 65])

    def test_to_sql_is_null(self):
        """Test to_sql for IS NULL."""
        expr = SqlExpr("middle_name", "IS NULL", None)
        sql, params = expr.to_sql()

        expect(sql).to_equal("middle_name IS NULL")
        expect(params).to_equal([])

    def test_to_sql_is_not_null(self):
        """Test to_sql for IS NOT NULL."""
        expr = SqlExpr("email", "IS NOT NULL", None)
        sql, params = expr.to_sql()

        expect(sql).to_equal("email IS NOT NULL")
        expect(params).to_equal([])

    def test_to_sql_custom_param_index(self):
        """Test to_sql with custom parameter index."""
        expr = SqlExpr("age", "=", 25)
        sql, params = expr.to_sql(param_index=5)

        expect(sql).to_equal("age = $5")
        expect(params).to_equal([25])


class TestSqlExprCombining:
    """Test combining SqlExpr with AND/OR."""

    def test_and_operator(self):
        """Test & (AND) operator."""
        expr1 = SqlExpr("age", ">", 18)
        expr2 = SqlExpr("age", "<", 65)

        combined = expr1 & expr2

        expect(isinstance(combined, SqlExpr)).to_be_true()
        expect(combined.op).to_equal("AND")
        expect(combined.value).to_equal([expr1, expr2])

    def test_or_operator(self):
        """Test | (OR) operator."""
        expr1 = SqlExpr("city", "=", "NYC")
        expr2 = SqlExpr("city", "=", "LA")

        combined = expr1 | expr2

        expect(isinstance(combined, SqlExpr)).to_be_true()
        expect(combined.op).to_equal("OR")
        expect(combined.value).to_equal([expr1, expr2])

    def test_invalid_and_type(self):
        """Test & raises TypeError for invalid type."""
        expr = SqlExpr("age", ">", 18)

        with pytest.raises(TypeError):
            _ = expr & "invalid"

    def test_invalid_or_type(self):
        """Test | raises TypeError for invalid type."""
        expr = SqlExpr("age", ">", 18)

        with pytest.raises(TypeError):
            _ = expr | 123


class TestSqlExprRepr:
    """Test SqlExpr __repr__."""

    def test_repr(self):
        """Test __repr__ returns useful string."""
        expr = SqlExpr("age", ">", 25)
        repr_str = repr(expr)

        expect("SqlExpr" in repr_str).to_be_true()
        expect("age" in repr_str).to_be_true()
        expect(">" in repr_str).to_be_true()
        expect("25" in repr_str).to_be_true()


class TestColumn:
    """Test Column descriptor."""

    def test_column_creation_basic(self):
        """Test basic Column creation."""
        col = Column()

        expect(col.default).to_be_none()
        expect(col.default_factory).to_be_none()
        expect(col.unique).to_equal(False)
        expect(col.index).to_equal(False)
        expect(col.nullable).to_equal(True)
        expect(col.primary_key).to_equal(False)

    def test_column_with_default(self):
        """Test Column with default value."""
        col = Column(default=0)

        expect(col.default).to_equal(0)

    def test_column_with_default_factory(self):
        """Test Column with default_factory."""

        def make_list():
            return []

        col = Column(default_factory=make_list)

        expect(col.default_factory).to_equal(make_list)

    def test_column_with_constraints(self):
        """Test Column with constraints."""
        col = Column(unique=True, index=True, nullable=False)

        expect(col.unique).to_equal(True)
        expect(col.index).to_equal(True)
        expect(col.nullable).to_equal(False)

    def test_column_primary_key(self):
        """Test Column as primary key."""
        col = Column(primary_key=True)

        expect(col.primary_key).to_equal(True)

    def test_column_with_description(self):
        """Test Column with description."""
        col = Column(description="User's email address")

        expect(col.description).to_equal("User's email address")

    def test_column_repr(self):
        """Test Column __repr__."""
        col = Column(default=0, unique=True)
        repr_str = repr(col)

        expect("Column" in repr_str).to_be_true()
        expect("default=0" in repr_str).to_be_true()
        expect("unique=True" in repr_str).to_be_true()

    def test_column_in_table(self):
        """Test Column used in Table definition."""

        class User(Table):
            email: str = Column(unique=True)
            age: int = Column(default=0)

        # Column should be captured in defaults
        expect("email" in User._column_defaults).to_be_true()
        expect("age" in User._column_defaults).to_be_true()
        expect(isinstance(User._column_defaults["email"], Column)).to_be_true()
        expect(isinstance(User._column_defaults["age"], Column)).to_be_true()

    def test_column_default_applied(self):
        """Test Column default is applied to instance."""

        class User(Table):
            name: str
            age: int = Column(default=18)

        user = User(name="Alice")

        expect(user.age).to_equal(18)

    def test_column_default_factory_applied(self):
        """Test Column default_factory is applied."""

        def make_tags():
            return ["user"]

        class User(Table):
            name: str
            tags: list = Column(default_factory=make_tags)

        user = User(name="Alice")

        expect(user.tags).to_equal(["user"])
