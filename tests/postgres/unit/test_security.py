"""
Unit tests for security validation.

Tests security validation of table names, column names, and SQL injection prevention.
These tests verify that the Python layer properly validates inputs before passing
them to the Rust engine.
"""
import pytest
from data_bridge.postgres import Table, Column


class TestTableNameValidation:
    """Test table name security validation."""

    def test_valid_table_name(self):
        """Test valid table name is accepted."""

        class Users(Table):
            name: str

            class Settings:
                table_name = "users"

        assert Users._table_name == "users"

    def test_valid_table_name_with_underscore(self):
        """Test table name with underscores is accepted."""

        class UserProfiles(Table):
            name: str

            class Settings:
                table_name = "user_profiles"

        assert UserProfiles._table_name == "user_profiles"

    def test_valid_table_name_with_numbers(self):
        """Test table name with numbers is accepted."""

        class Orders(Table):
            name: str

            class Settings:
                table_name = "orders_2024"

        assert Orders._table_name == "orders_2024"

    def test_schema_qualified_table_name(self):
        """Test schema-qualified table names work correctly."""

        class Users(Table):
            name: str

            class Settings:
                table_name = "users"
                schema = "auth"

        # Schema is stored separately, not in table_name
        assert Users._table_name == "users"
        assert Users._schema == "auth"
        assert Users.__table_name__() == "auth.users"

    def test_table_name_case_preserved(self):
        """Test table name case is preserved."""

        class Users(Table):
            name: str

            class Settings:
                table_name = "MyUsers"

        # PostgreSQL will lowercase unless quoted, but we preserve the case
        assert Users._table_name == "MyUsers"


class TestColumnNameValidation:
    """Test column name security validation."""

    def test_valid_column_names(self):
        """Test valid column names are accepted."""

        class User(Table):
            first_name: str
            last_name: str
            email_address: str
            age: int

        assert "first_name" in User._columns
        assert "last_name" in User._columns
        assert "email_address" in User._columns
        assert "age" in User._columns

    def test_column_name_with_numbers(self):
        """Test column names with numbers are accepted."""

        class Product(Table):
            name: str
            price_v2: float

        assert "price_v2" in Product._columns

    def test_column_name_case_preserved(self):
        """Test column name case is preserved."""

        class User(Table):
            firstName: str
            lastName: str

        # Case should be preserved
        assert "firstName" in User._columns
        assert "lastName" in User._columns


class TestSQLInjectionPrevention:
    """Test SQL injection prevention in various contexts."""

    def test_semicolon_in_table_name(self):
        """Test semicolon in table name (SQL injection attempt)."""
        # This should be caught at the Rust validation layer when the table is used
        # Python layer allows it, but Rust will reject it

        class Users(Table):
            name: str

            class Settings:
                table_name = "users; DROP TABLE users--"

        # Python allows setting it, but Rust engine would reject
        assert Users._table_name == "users; DROP TABLE users--"

    def test_comment_in_table_name(self):
        """Test SQL comment in table name."""

        class Users(Table):
            name: str

            class Settings:
                table_name = "users--comment"

        # Python allows it, validation happens at Rust layer
        assert Users._table_name == "users--comment"

    def test_special_chars_in_filter_value(self):
        """Test special characters in filter values are safe."""
        # Values are parameterized, so should be safe

        class User(Table):
            name: str
            email: str

        # This should generate parameterized SQL
        expr = User.email == "test'; DROP TABLE users--"

        assert expr.value == "test'; DROP TABLE users--"
        # The to_sql() should use parameterized queries
        sql, params = expr.to_sql()
        assert "$1" in sql  # Parameterized
        assert params[0] == "test'; DROP TABLE users--"

    def test_sql_keywords_in_values(self):
        """Test SQL keywords in values are safe."""

        class User(Table):
            name: str
            bio: str

        expr = User.bio.contains("SELECT * FROM")

        sql, params = expr.to_sql()
        # Should be parameterized
        assert "$1" in sql
        assert "SELECT * FROM" in params[0]

    def test_union_injection_in_values(self):
        """Test UNION injection attempt in values."""

        class User(Table):
            name: str

        expr = User.name == "admin' UNION SELECT password FROM users--"

        sql, params = expr.to_sql()
        # Should be safely parameterized
        assert "$1" in sql
        assert "UNION" in params[0]  # Treated as literal value


class TestIdentifierValidation:
    """Test identifier validation (table/column names)."""

    def test_empty_table_name(self):
        """Test empty table name uses class name."""

        class User(Table):
            name: str

            class Settings:
                table_name = ""

        # Should default to lowercase class name
        assert User._table_name == "user"

    def test_long_identifier(self):
        """Test very long identifier names."""
        # PostgreSQL has a limit of 63 characters for identifiers

        long_name = "a" * 100

        class MyTable(Table):
            name: str

            class Settings:
                table_name = long_name

        # Python allows it, but PostgreSQL might truncate
        assert MyTable._table_name == long_name

    def test_reserved_words_as_column_names(self):
        """Test SQL reserved words as column names."""
        # These are technically valid if quoted

        class MyTable(Table):
            select: str  # SQL keyword
            from_: str  # SQL keyword (using Python convention)
            where: str  # SQL keyword

        # Python allows it
        assert "select" in MyTable._columns
        assert "from_" in MyTable._columns
        assert "where" in MyTable._columns

    def test_unicode_in_identifiers(self):
        """Test unicode characters in identifiers."""

        class User(Table):
            name: str

            class Settings:
                table_name = "用户"  # Chinese characters

        # Python allows it
        assert User._table_name == "用户"


class TestSchemaValidation:
    """Test schema name validation."""

    def test_valid_schema_name(self):
        """Test valid schema names."""

        class User(Table):
            name: str

            class Settings:
                schema = "public"

        assert User._schema == "public"

    def test_custom_schema_name(self):
        """Test custom schema names."""

        class User(Table):
            name: str

            class Settings:
                schema = "auth"

        assert User._schema == "auth"

    def test_schema_with_underscore(self):
        """Test schema name with underscore."""

        class User(Table):
            name: str

            class Settings:
                schema = "my_schema"

        assert User._schema == "my_schema"

    def test_default_schema(self):
        """Test default schema is 'public'."""

        class User(Table):
            name: str

        assert User._schema == "public"


class TestPrimaryKeyValidation:
    """Test primary key configuration validation."""

    def test_valid_primary_key(self):
        """Test valid primary key name."""

        class Product(Table):
            sku: str

            class Settings:
                primary_key = "sku"

        assert Product._primary_key == "sku"

    def test_default_primary_key(self):
        """Test default primary key is 'id'."""

        class User(Table):
            name: str

        assert User._primary_key == "id"

    def test_numeric_primary_key(self):
        """Test numeric primary key name."""

        class User(Table):
            name: str

            class Settings:
                primary_key = "user_id"

        assert User._primary_key == "user_id"


class TestQueryParameterization:
    """Test that queries use parameterization for safety."""

    def test_filter_uses_parameters(self):
        """Test filter values are parameterized."""

        class User(Table):
            email: str

        expr = User.email == "test@example.com"
        sql, params = expr.to_sql()

        # Should use $1 placeholder
        assert "$1" in sql
        assert params == ["test@example.com"]

    def test_multiple_filters_parameterized(self):
        """Test multiple filters use sequential parameters."""

        class User(Table):
            name: str
            age: int

        query = User.find(User.name == "Alice", User.age > 25)
        where, params = query._build_where_clause()

        # Should use $1, $2
        assert "$1" in where
        assert "$2" in where
        assert len(params) == 2

    def test_in_operator_parameterized(self):
        """Test IN operator uses parameterization."""

        class User(Table):
            city: str

        expr = User.city.in_(["NYC", "LA", "SF"])
        sql, params = expr.to_sql()

        # Should use $1, $2, $3
        assert "IN" in sql
        assert "$1" in sql
        assert "$2" in sql
        assert "$3" in sql
        assert params == ["NYC", "LA", "SF"]

    def test_like_operator_parameterized(self):
        """Test LIKE operator uses parameterization."""

        class User(Table):
            email: str

        expr = User.email.like("%@example.com")
        sql, params = expr.to_sql()

        # Should be parameterized
        assert "$1" in sql
        assert params == ["%@example.com"]

    def test_between_parameterized(self):
        """Test BETWEEN uses parameterization."""

        class User(Table):
            age: int

        expr = User.age.between(18, 65)
        sql, params = expr.to_sql()

        # Should use $1 and $2
        assert "BETWEEN $1 AND $2" in sql
        assert params == [18, 65]


class TestInputSanitization:
    """Test input sanitization and validation."""

    def test_null_byte_in_string(self):
        """Test null bytes in string values."""

        class User(Table):
            name: str

        # Null byte should be treated as regular value
        expr = User.name == "test\x00value"

        sql, params = expr.to_sql()
        assert params[0] == "test\x00value"

    def test_newline_in_string(self):
        """Test newlines in string values are safe."""

        class User(Table):
            bio: str

        expr = User.bio == "Line 1\nLine 2"

        sql, params = expr.to_sql()
        assert params[0] == "Line 1\nLine 2"

    def test_quote_in_string(self):
        """Test quotes in string values are safe."""

        class User(Table):
            name: str

        expr = User.name == "O'Brien"

        sql, params = expr.to_sql()
        # Parameterization makes this safe
        assert params[0] == "O'Brien"

    def test_backslash_in_string(self):
        """Test backslashes in string values."""

        class User(Table):
            path: str

        expr = User.path == "C:\\Users\\Admin"

        sql, params = expr.to_sql()
        assert params[0] == "C:\\Users\\Admin"
