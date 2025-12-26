"""
Unit tests for Column descriptors and SQL expressions.

Tests ColumnProxy, SqlExpr, and Column classes.
"""
import pytest
from data_bridge.postgres import Table, Column
from data_bridge.postgres.columns import ColumnProxy, SqlExpr


class TestColumnProxy:
    """Test ColumnProxy descriptor behavior."""

    def test_column_proxy_creation(self):
        """Test ColumnProxy can be created."""
        proxy = ColumnProxy("email")

        assert proxy.name == "email"
        assert proxy.model is None

    def test_column_proxy_with_model(self):
        """Test ColumnProxy with model reference."""

        class User(Table):
            email: str

        proxy = User.email

        assert isinstance(proxy, ColumnProxy)
        assert proxy.name == "email"
        assert proxy.model == User

    def test_column_proxy_repr(self):
        """Test ColumnProxy __repr__."""
        proxy = ColumnProxy("email")
        repr_str = repr(proxy)

        assert "ColumnProxy" in repr_str
        assert "email" in repr_str

    def test_column_proxy_hashable(self):
        """Test ColumnProxy is hashable (can be dict key)."""

        class User(Table):
            email: str

        proxy = User.email

        # Should be usable as dict key
        d = {proxy: "value"}
        assert d[proxy] == "value"

    def test_descriptor_class_access(self):
        """Test descriptor returns self on class access."""

        class User(Table):
            email: str

        # Class access should return ColumnProxy
        assert isinstance(User.email, ColumnProxy)

    def test_descriptor_instance_access(self):
        """Test descriptor returns value on instance access."""

        class User(Table):
            email: str

        user = User(email="test@example.com")

        # Instance access should return value
        assert user.email == "test@example.com"

    def test_descriptor_set(self):
        """Test descriptor __set__ updates _data."""

        class User(Table):
            email: str

        user = User(email="old@example.com")
        user.email = "new@example.com"

        assert user._data["email"] == "new@example.com"


class TestSqlExprCreation:
    """Test SqlExpr creation from operators."""

    def test_equals(self):
        """Test == creates SqlExpr."""

        class User(Table):
            age: int

        expr = User.age == 25

        assert isinstance(expr, SqlExpr)
        assert expr.column == "age"
        assert expr.op == "="
        assert expr.value == 25

    def test_not_equals(self):
        """Test != creates SqlExpr."""

        class User(Table):
            status: str

        expr = User.status != "deleted"

        assert expr.op == "!="
        assert expr.value == "deleted"

    def test_greater_than(self):
        """Test > creates SqlExpr."""

        class User(Table):
            age: int

        expr = User.age > 18

        assert expr.op == ">"
        assert expr.value == 18

    def test_greater_equal(self):
        """Test >= creates SqlExpr."""

        class User(Table):
            score: int

        expr = User.score >= 100

        assert expr.op == ">="

    def test_less_than(self):
        """Test < creates SqlExpr."""

        class User(Table):
            age: int

        expr = User.age < 65

        assert expr.op == "<"

    def test_less_equal(self):
        """Test <= creates SqlExpr."""

        class User(Table):
            age: int

        expr = User.age <= 100

        assert expr.op == "<="


class TestSqlExprMethods:
    """Test SqlExpr methods (in_, like, etc.)."""

    def test_in_method(self):
        """Test in_() method."""

        class User(Table):
            city: str

        expr = User.city.in_(["NYC", "LA", "SF"])

        assert expr.op == "IN"
        assert expr.value == ["NYC", "LA", "SF"]

    def test_between_method(self):
        """Test between() method."""

        class User(Table):
            age: int

        expr = User.age.between(18, 65)

        assert expr.op == "BETWEEN"
        assert expr.value == [18, 65]

    def test_is_null_method(self):
        """Test is_null() method."""

        class User(Table):
            middle_name: str

        expr = User.middle_name.is_null()

        assert expr.op == "IS NULL"
        assert expr.value is None

    def test_is_not_null_method(self):
        """Test is_not_null() method."""

        class User(Table):
            email: str

        expr = User.email.is_not_null()

        assert expr.op == "IS NOT NULL"

    def test_like_method(self):
        """Test like() method."""

        class User(Table):
            email: str

        expr = User.email.like("%@example.com")

        assert expr.op == "LIKE"
        assert expr.value == "%@example.com"

    def test_ilike_method(self):
        """Test ilike() case-insensitive method."""

        class User(Table):
            email: str

        expr = User.email.ilike("%@EXAMPLE.COM")

        assert expr.op == "ILIKE"
        assert expr.value == "%@EXAMPLE.COM"

    def test_startswith_method(self):
        """Test startswith() convenience method."""

        class User(Table):
            name: str

        expr = User.name.startswith("A")

        assert expr.op == "LIKE"
        assert expr.value == "A%"

    def test_contains_method(self):
        """Test contains() convenience method."""

        class User(Table):
            bio: str

        expr = User.bio.contains("python")

        assert expr.op == "LIKE"
        assert expr.value == "%python%"


class TestSqlExprToSQL:
    """Test SqlExpr to_sql() conversion."""

    def test_to_sql_equals(self):
        """Test to_sql for equals operator."""
        expr = SqlExpr("age", "=", 25)
        sql, params = expr.to_sql()

        assert sql == "age = $1"
        assert params == [25]

    def test_to_sql_greater_than(self):
        """Test to_sql for greater than."""
        expr = SqlExpr("age", ">", 18)
        sql, params = expr.to_sql()

        assert sql == "age > $1"
        assert params == [18]

    def test_to_sql_in(self):
        """Test to_sql for IN operator."""
        expr = SqlExpr("city", "IN", ["NYC", "LA", "SF"])
        sql, params = expr.to_sql()

        assert "city IN" in sql
        assert "$1" in sql
        assert "$2" in sql
        assert "$3" in sql
        assert params == ["NYC", "LA", "SF"]

    def test_to_sql_between(self):
        """Test to_sql for BETWEEN."""
        expr = SqlExpr("age", "BETWEEN", [18, 65])
        sql, params = expr.to_sql()

        assert "age BETWEEN $1 AND $2" in sql
        assert params == [18, 65]

    def test_to_sql_is_null(self):
        """Test to_sql for IS NULL."""
        expr = SqlExpr("middle_name", "IS NULL", None)
        sql, params = expr.to_sql()

        assert sql == "middle_name IS NULL"
        assert params == []

    def test_to_sql_is_not_null(self):
        """Test to_sql for IS NOT NULL."""
        expr = SqlExpr("email", "IS NOT NULL", None)
        sql, params = expr.to_sql()

        assert sql == "email IS NOT NULL"
        assert params == []

    def test_to_sql_custom_param_index(self):
        """Test to_sql with custom parameter index."""
        expr = SqlExpr("age", "=", 25)
        sql, params = expr.to_sql(param_index=5)

        assert sql == "age = $5"
        assert params == [25]


class TestSqlExprCombining:
    """Test combining SqlExpr with AND/OR."""

    def test_and_operator(self):
        """Test & (AND) operator."""
        expr1 = SqlExpr("age", ">", 18)
        expr2 = SqlExpr("age", "<", 65)

        combined = expr1 & expr2

        assert isinstance(combined, SqlExpr)
        assert combined.op == "AND"
        assert combined.value == [expr1, expr2]

    def test_or_operator(self):
        """Test | (OR) operator."""
        expr1 = SqlExpr("city", "=", "NYC")
        expr2 = SqlExpr("city", "=", "LA")

        combined = expr1 | expr2

        assert isinstance(combined, SqlExpr)
        assert combined.op == "OR"
        assert combined.value == [expr1, expr2]

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

        assert "SqlExpr" in repr_str
        assert "age" in repr_str
        assert ">" in repr_str
        assert "25" in repr_str


class TestColumn:
    """Test Column descriptor."""

    def test_column_creation_basic(self):
        """Test basic Column creation."""
        col = Column()

        assert col.default is None
        assert col.default_factory is None
        assert col.unique is False
        assert col.index is False
        assert col.nullable is True
        assert col.primary_key is False

    def test_column_with_default(self):
        """Test Column with default value."""
        col = Column(default=0)

        assert col.default == 0

    def test_column_with_default_factory(self):
        """Test Column with default_factory."""

        def make_list():
            return []

        col = Column(default_factory=make_list)

        assert col.default_factory == make_list

    def test_column_with_constraints(self):
        """Test Column with constraints."""
        col = Column(unique=True, index=True, nullable=False)

        assert col.unique is True
        assert col.index is True
        assert col.nullable is False

    def test_column_primary_key(self):
        """Test Column as primary key."""
        col = Column(primary_key=True)

        assert col.primary_key is True

    def test_column_with_description(self):
        """Test Column with description."""
        col = Column(description="User's email address")

        assert col.description == "User's email address"

    def test_column_repr(self):
        """Test Column __repr__."""
        col = Column(default=0, unique=True)
        repr_str = repr(col)

        assert "Column" in repr_str
        assert "default=0" in repr_str
        assert "unique=True" in repr_str

    def test_column_in_table(self):
        """Test Column used in Table definition."""

        class User(Table):
            email: str = Column(unique=True)
            age: int = Column(default=0)

        # Column should be captured in defaults
        assert "email" in User._column_defaults
        assert "age" in User._column_defaults
        assert isinstance(User._column_defaults["email"], Column)
        assert isinstance(User._column_defaults["age"], Column)

    def test_column_default_applied(self):
        """Test Column default is applied to instance."""

        class User(Table):
            name: str
            age: int = Column(default=18)

        user = User(name="Alice")

        assert user.age == 18

    def test_column_default_factory_applied(self):
        """Test Column default_factory is applied."""

        def make_tags():
            return ["user"]

        class User(Table):
            name: str
            tags: list = Column(default_factory=make_tags)

        user = User(name="Alice")

        assert user.tags == ["user"]
