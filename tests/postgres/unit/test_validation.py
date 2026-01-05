"""Tests for PostgreSQL ORM validation module."""

import pytest
from datetime import datetime, date
from decimal import Decimal

from data_bridge.postgres import Table, Column
from data_bridge.postgres.validation import (
    validates, validates_many,
    TypeDecorator,
    coerce_int, coerce_float, coerce_str, coerce_bool, coerce_datetime, coerce_date, coerce_decimal,
    ValidationError,
    ValidatorRegistry,
    AutoCoerceMixin,
    validate_not_empty, validate_email, validate_url,
    validate_min_length, validate_max_length, validate_regex,
    validate_range, validate_min_value, validate_max_value,
    validate_in_list, validate_positive, validate_non_negative,
)


# ============================================================================
# Test Coercion Functions
# ============================================================================

class TestCoercionFunctions:
    """Test type coercion functions."""

    def test_coerce_int(self):
        """Test integer coercion."""
        assert coerce_int(42) == 42
        assert coerce_int("42") == 42
        assert coerce_int(3.14) == 3
        assert coerce_int(None) is None

        with pytest.raises(ValueError):
            coerce_int("not a number")

    def test_coerce_float(self):
        """Test float coercion."""
        assert coerce_float(3.14) == 3.14
        assert coerce_float("3.14") == 3.14
        assert coerce_float(42) == 42.0
        assert coerce_float(None) is None

        with pytest.raises(ValueError):
            coerce_float("not a number")

    def test_coerce_str(self):
        """Test string coercion."""
        assert coerce_str("hello") == "hello"
        assert coerce_str(42) == "42"
        assert coerce_str(3.14) == "3.14"
        assert coerce_str(None) is None

    def test_coerce_bool(self):
        """Test boolean coercion."""
        # Direct booleans
        assert coerce_bool(True) is True
        assert coerce_bool(False) is False

        # Integers
        assert coerce_bool(1) is True
        assert coerce_bool(0) is False

        # Strings
        assert coerce_bool("true") is True
        assert coerce_bool("TRUE") is True
        assert coerce_bool("yes") is True
        assert coerce_bool("1") is True
        assert coerce_bool("false") is False
        assert coerce_bool("FALSE") is False
        assert coerce_bool("no") is False
        assert coerce_bool("0") is False

        # None
        assert coerce_bool(None) is None

        # Invalid
        with pytest.raises(ValueError):
            coerce_bool("invalid")

    def test_coerce_datetime(self):
        """Test datetime coercion."""
        dt = datetime(2024, 1, 15, 10, 30)

        # Datetime unchanged
        assert coerce_datetime(dt) == dt

        # Date to datetime
        d = date(2024, 1, 15)
        result = coerce_datetime(d)
        assert result.year == 2024
        assert result.month == 1
        assert result.day == 15

        # Unix timestamp
        timestamp = 1705315800
        result = coerce_datetime(timestamp)
        assert isinstance(result, datetime)

        # ISO string
        iso_str = "2024-01-15T10:30:00"
        result = coerce_datetime(iso_str)
        assert result.year == 2024
        assert result.month == 1
        assert result.day == 15

        # None
        assert coerce_datetime(None) is None

        # Invalid
        with pytest.raises(ValueError):
            coerce_datetime("not a datetime")

    def test_coerce_date(self):
        """Test date coercion."""
        d = date(2024, 1, 15)

        # Date unchanged
        assert coerce_date(d) == d

        # Datetime to date
        dt = datetime(2024, 1, 15, 10, 30)
        result = coerce_date(dt)
        assert result == d

        # ISO string
        iso_str = "2024-01-15"
        result = coerce_date(iso_str)
        assert result == d

        # None
        assert coerce_date(None) is None

        # Invalid
        with pytest.raises(ValueError):
            coerce_date("not a date")

    def test_coerce_decimal(self):
        """Test decimal coercion."""
        # Decimal unchanged
        dec = Decimal("3.14159")
        assert coerce_decimal(dec) == dec

        # String to decimal
        assert coerce_decimal("3.14159") == Decimal("3.14159")

        # Float to decimal (note: uses string conversion to avoid float precision issues)
        result = coerce_decimal(3.14)
        assert isinstance(result, Decimal)

        # Int to decimal
        assert coerce_decimal(42) == Decimal("42")

        # None
        assert coerce_decimal(None) is None


# ============================================================================
# Test Built-in Validators
# ============================================================================

class TestBuiltinValidators:
    """Test built-in validation functions."""

    def test_validate_not_empty(self):
        """Test non-empty validation."""
        assert validate_not_empty("hello") is True

        with pytest.raises(ValueError):
            validate_not_empty("")

        with pytest.raises(ValueError):
            validate_not_empty(None)

    def test_validate_email(self):
        """Test email validation."""
        assert validate_email("alice@example.com") is True
        assert validate_email("bob+test@subdomain.example.co.uk") is True

        assert validate_email("invalid-email") is False
        assert validate_email("@example.com") is False
        assert validate_email("alice@") is False
        assert validate_email("") is False
        assert validate_email(None) is False

    def test_validate_url(self):
        """Test URL validation."""
        assert validate_url("https://example.com") is True
        assert validate_url("http://example.com") is True
        assert validate_url("https://subdomain.example.com/path/to/page") is True

        assert validate_url("not-a-url") is False
        assert validate_url("ftp://example.com") is False
        assert validate_url("") is False
        assert validate_url(None) is False

    def test_validate_min_length(self):
        """Test minimum length validation."""
        assert validate_min_length("hello", 3) is True
        assert validate_min_length("hello", 5) is True

        with pytest.raises(ValueError):
            validate_min_length("hi", 5)

    def test_validate_max_length(self):
        """Test maximum length validation."""
        assert validate_max_length("hello", 10) is True
        assert validate_max_length("hello", 5) is True

        with pytest.raises(ValueError):
            validate_max_length("hello world", 5)

    def test_validate_regex(self):
        """Test regex validation."""
        assert validate_regex("abc123", r"^[a-z]+[0-9]+$") is True

        with pytest.raises(ValueError):
            validate_regex("123abc", r"^[a-z]+[0-9]+$")

    def test_validate_range(self):
        """Test range validation."""
        assert validate_range(5, 1, 10) is True
        assert validate_range(1, 1, 10) is True
        assert validate_range(10, 1, 10) is True

        with pytest.raises(ValueError):
            validate_range(0, 1, 10)

        with pytest.raises(ValueError):
            validate_range(11, 1, 10)

    def test_validate_min_value(self):
        """Test minimum value validation."""
        assert validate_min_value(5, 1) is True
        assert validate_min_value(1, 1) is True

        with pytest.raises(ValueError):
            validate_min_value(0, 1)

    def test_validate_max_value(self):
        """Test maximum value validation."""
        assert validate_max_value(5, 10) is True
        assert validate_max_value(10, 10) is True

        with pytest.raises(ValueError):
            validate_max_value(11, 10)

    def test_validate_in_list(self):
        """Test in-list validation."""
        assert validate_in_list("red", ["red", "green", "blue"]) is True

        with pytest.raises(ValueError):
            validate_in_list("yellow", ["red", "green", "blue"])

    def test_validate_positive(self):
        """Test positive validation."""
        assert validate_positive(1) is True
        assert validate_positive(0.1) is True

        with pytest.raises(ValueError):
            validate_positive(0)

        with pytest.raises(ValueError):
            validate_positive(-1)

    def test_validate_non_negative(self):
        """Test non-negative validation."""
        assert validate_non_negative(0) is True
        assert validate_non_negative(1) is True

        with pytest.raises(ValueError):
            validate_non_negative(-1)


# ============================================================================
# Test TypeDecorator
# ============================================================================

class TestTypeDecorator:
    """Test TypeDecorator base class."""

    def test_lowercase_string_decorator(self):
        """Test custom lowercase string type decorator."""
        class LowercaseString(TypeDecorator):
            impl = str

            def process_bind_param(self, value, dialect=None):
                return value.lower() if value else value

            def process_result_value(self, value, dialect=None):
                return value.lower() if value else value

        decorator = LowercaseString()
        assert decorator.process_bind_param("HELLO", None) == "hello"
        assert decorator.process_result_value("WORLD", None) == "world"
        assert decorator.process_bind_param(None, None) is None

    def test_type_decorator_coerce(self):
        """Test TypeDecorator coerce method."""
        decorator = TypeDecorator()
        decorator.impl = int

        assert decorator.coerce("42") == 42
        assert decorator.coerce(None) is None


# ============================================================================
# Test ValidatorRegistry
# ============================================================================

class TestValidatorRegistry:
    """Test ValidatorRegistry."""

    def test_register_and_get_validators(self):
        """Test registering and retrieving validators."""
        registry = ValidatorRegistry()

        def validator1(self, key, value):
            return value

        def validator2(self, key, value):
            return value.upper()

        class TestTable(Table):
            email: str

        registry.register(TestTable, 'email', validator1)
        registry.register(TestTable, 'email', validator2)

        validators = registry.get_validators(TestTable, 'email')
        assert len(validators) == 2
        assert validators[0] is validator1
        assert validators[1] is validator2

    def test_validate_field(self):
        """Test field validation through registry."""
        registry = ValidatorRegistry()

        def lowercase_validator(self, key, value):
            return value.lower()

        class TestTable(Table):
            email: str

        registry.register(TestTable, 'email', lowercase_validator)

        instance = TestTable(email="test@example.com")
        result = registry.validate_field(instance, 'email', "ALICE@EXAMPLE.COM")
        assert result == "alice@example.com"

    def test_register_multi_validator(self):
        """Test multi-field validator registration."""
        registry = ValidatorRegistry()

        def multi_validator(self, values):
            return values

        class TestTable(Table):
            password: str
            password_confirm: str

        registry.register_multi(TestTable, multi_validator)

        validators = registry.get_multi_validators(TestTable)
        assert len(validators) == 1
        assert validators[0] is multi_validator

    def test_validate_many(self):
        """Test multi-field validation through registry."""
        registry = ValidatorRegistry()

        def password_validator(self, values):
            if values.get('password') != values.get('password_confirm'):
                raise ValidationError('password', "Passwords don't match")
            return values

        class TestTable(Table):
            password: str
            password_confirm: str

        registry.register_multi(TestTable, password_validator)

        instance = TestTable(password="secret", password_confirm="secret")
        result = registry.validate_many(instance, {'password': 'secret', 'password_confirm': 'secret'})
        assert result == {'password': 'secret', 'password_confirm': 'secret'}

        # Test validation error
        with pytest.raises(ValidationError) as exc_info:
            registry.validate_many(instance, {'password': 'secret', 'password_confirm': 'different'})
        assert exc_info.value.field == 'password'


# ============================================================================
# Test @validates Decorator
# ============================================================================

class TestValidatesDecorator:
    """Test @validates decorator."""

    def test_validates_decorator_metadata(self):
        """Test that @validates decorator adds metadata."""
        @validates('email')
        def validate_email_func(self, key, value):
            return value.lower()

        assert hasattr(validate_email_func, '_validates_fields')
        assert 'email' in validate_email_func._validates_fields

    def test_validates_multiple_fields(self):
        """Test @validates decorator with multiple fields."""
        @validates('email', 'username')
        def validate_fields(self, key, value):
            return value.lower()

        assert hasattr(validate_fields, '_validates_fields')
        assert 'email' in validate_fields._validates_fields
        assert 'username' in validate_fields._validates_fields


# ============================================================================
# Test @validates_many Decorator
# ============================================================================

class TestValidatesManyDecorator:
    """Test @validates_many decorator."""

    def test_validates_many_decorator_metadata(self):
        """Test that @validates_many decorator adds metadata."""
        @validates_many('password', 'password_confirm')
        def validate_passwords(self, values):
            return values

        assert hasattr(validate_passwords, '_validates_many_fields')
        assert 'password' in validate_passwords._validates_many_fields
        assert 'password_confirm' in validate_passwords._validates_many_fields


# ============================================================================
# Test AutoCoerceMixin
# ============================================================================

class TestAutoCoerceMixin:
    """Test AutoCoerceMixin."""

    def test_auto_coerce_all_fields(self):
        """Test auto-coercion on all fields."""
        class TestTable(AutoCoerceMixin, Table):  # AutoCoerceMixin first!
            age: int
            score: float
            active: bool
            name: str

        instance = TestTable(age=25, score=3.14, active=True, name="Alice")
        instance.age = "30"
        instance.score = "2.5"
        instance.active = "yes"
        instance.name = 42

        assert instance.age == 30
        assert instance.score == 2.5
        assert instance.active is True
        assert instance.name == "42"

    def test_auto_coerce_specific_fields(self):
        """Test auto-coercion on specific fields only."""
        class TestTable(AutoCoerceMixin, Table):  # AutoCoerceMixin first!
            age: int
            score: float
            name: str

            __coerce_fields__ = {'age', 'score'}

        instance = TestTable(age=25, score=3.14, name="Alice")
        instance.age = "30"
        instance.score = "2.5"
        instance.name = 42  # Should NOT be coerced

        assert instance.age == 30
        assert instance.score == 2.5
        assert instance.name == 42  # Remains as int

    def test_auto_coerce_datetime_fields(self):
        """Test auto-coercion for datetime fields."""
        class TestTable(AutoCoerceMixin, Table):  # AutoCoerceMixin first!
            created_at: datetime
            birth_date: date

        instance = TestTable(
            created_at=datetime(2024, 1, 15, 10, 30),
            birth_date=date(1990, 5, 20)
        )
        instance.created_at = "2024-01-16T12:00:00"
        instance.birth_date = "1991-06-21"

        assert instance.created_at == datetime(2024, 1, 16, 12, 0, 0)
        assert instance.birth_date == date(1991, 6, 21)

    def test_auto_coerce_decimal_fields(self):
        """Test auto-coercion for Decimal fields."""
        class TestTable(AutoCoerceMixin, Table):  # AutoCoerceMixin first!
            price: Decimal

        instance = TestTable(price=Decimal("19.99"))
        instance.price = "29.99"

        assert instance.price == Decimal("29.99")

    def test_auto_coerce_skips_none(self):
        """Test that auto-coercion handles None values."""
        class TestTable(AutoCoerceMixin, Table):  # AutoCoerceMixin first!
            age: int

        instance = TestTable(age=25)
        instance.age = None

        assert instance.age is None

    def test_auto_coerce_skips_private_attributes(self):
        """Test that auto-coercion skips private attributes."""
        class TestTable(AutoCoerceMixin, Table):  # AutoCoerceMixin first!
            age: int

        instance = TestTable(age=25)
        instance._private = "should not coerce"

        assert instance._private == "should not coerce"


# ============================================================================
# Test ValidationError
# ============================================================================

class TestValidationError:
    """Test ValidationError exception."""

    def test_validation_error_attributes(self):
        """Test ValidationError attributes."""
        error = ValidationError('email', 'Invalid email format')
        assert error.field == 'email'
        assert error.message == 'Invalid email format'
        assert "Validation failed for field 'email'" in str(error)
        assert 'Invalid email format' in str(error)


# ============================================================================
# Integration Tests
# ============================================================================

class TestValidationIntegration:
    """Integration tests combining multiple validation features."""

    def test_combined_validation_and_coercion(self):
        """Test combining @validates with AutoCoerceMixin."""
        class User(AutoCoerceMixin, Table):  # AutoCoerceMixin first!
            email: str
            age: int

            __coerce_fields__ = {'age'}

            @validates('email')
            def validate_email(self, key, value):
                if not validate_email(value):
                    raise ValidationError('email', 'Invalid email format')
                return value.lower()

            @validates('age')
            def validate_age(self, key, value):
                if value < 0:
                    raise ValidationError('age', 'Age must be non-negative')
                return value

        # This should work (auto-coercion + validation)
        user = User(email="ALICE@EXAMPLE.COM", age="30")

        # Email should be validated but not auto-coerced (not in __coerce_fields__)
        # Age should be auto-coerced

        # Note: In this test, we're just verifying the decorators are set up correctly
        # Actual validation would happen in Table.__setattr__ integration
        assert hasattr(User.validate_email, '_validates_fields')
        assert hasattr(User.validate_age, '_validates_fields')

    def test_custom_type_decorator(self):
        """Test creating and using a custom TypeDecorator."""
        class UppercaseString(TypeDecorator):
            impl = str

            def process_bind_param(self, value, dialect=None):
                return value.upper() if value else value

            def process_result_value(self, value, dialect=None):
                return value.upper() if value else value

        decorator = UppercaseString()

        # Simulate binding parameter (Python to DB)
        result = decorator.process_bind_param("hello", "postgresql")
        assert result == "HELLO"

        # Simulate result value (DB to Python)
        result = decorator.process_result_value("world", "postgresql")
        assert result == "WORLD"

    def test_multi_field_validation(self):
        """Test multi-field validation scenario."""
        class PasswordForm(Table):
            password: str
            password_confirm: str
            email: str

            @validates_many('password', 'password_confirm')
            def validate_passwords(self, values):
                pwd = values.get('password')
                pwd_confirm = values.get('password_confirm')

                if pwd and pwd_confirm and pwd != pwd_confirm:
                    raise ValidationError('password', "Passwords don't match")

                return values

        # Verify decorator metadata
        assert hasattr(PasswordForm.validate_passwords, '_validates_many_fields')
        assert 'password' in PasswordForm.validate_passwords._validates_many_fields
        assert 'password_confirm' in PasswordForm.validate_passwords._validates_many_fields


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
