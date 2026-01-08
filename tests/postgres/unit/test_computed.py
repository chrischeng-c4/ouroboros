"""
Unit tests for Computed Attributes module.

Tests hybrid_property, hybrid_method, column_property, Computed,
default_factory, and related descriptors without requiring a database.
"""
import pytest
from typing import Any
from unittest.mock import Mock, AsyncMock, patch
from data_bridge.test import expect
from data_bridge.postgres.computed import (
    hybrid_property,
    hybrid_method,
    column_property,
    Computed,
    ComputedColumn,
    default_factory,
    HybridPropertyDescriptor,
    HybridMethodDescriptor,
    ColumnPropertyDescriptor,
)
from data_bridge.postgres import Table


# Test fixtures

@pytest.fixture
def sample_table_class():
    """Sample Table class for testing computed attributes."""
    class User(Table):
        first_name: str
        last_name: str
        age: int = 0

        class Settings:
            table_name = "users"

    return User


@pytest.fixture
def sample_product_class():
    """Sample Product class for testing computed columns."""
    class Product(Table):
        price: float
        quantity: int

        class Settings:
            table_name = "products"

    return Product


# HybridPropertyDescriptor Tests

class TestHybridPropertyDescriptor:
    """Test HybridPropertyDescriptor class."""

    def test_descriptor_creation(self):
        """Test HybridPropertyDescriptor can be created."""
        def getter(self):
            return "test"

        descriptor = HybridPropertyDescriptor(getter)

        expect(descriptor.fget).to_equal(getter)
        expect(descriptor.fset).to_be_none()
        expect(descriptor.expr).to_be_none()
        expect(descriptor._name).to_be_none()

    def test_descriptor_copies_function_metadata(self):
        """Test descriptor copies __doc__ and __name__ from function."""
        def my_property(self):
            """Property docstring."""
            return "value"

        descriptor = HybridPropertyDescriptor(my_property)

        expect(descriptor.__doc__).to_equal("Property docstring.")
        expect(descriptor.__name__).to_equal("my_property")

    def test_set_name_assigns_name(self):
        """Test __set_name__ assigns the attribute name."""
        def getter(self):
            return "test"

        descriptor = HybridPropertyDescriptor(getter)
        descriptor.__set_name__(Mock, "full_name")

        expect(descriptor._name).to_equal("full_name")

    def test_instance_access_calls_getter(self):
        """Test instance access calls the getter function."""
        class TestClass:
            def __init__(self):
                self.first = "John"
                self.last = "Doe"

            @hybrid_property
            def full_name(self):
                return f"{self.first} {self.last}"

        instance = TestClass()

        expect(instance.full_name).to_equal("John Doe")

    def test_class_access_returns_sql_expression(self):
        """Test class access returns SQL expression when defined."""
        class MockSqlExpr:
            def __init__(self, expr):
                self.expr = expr

        class TestClass:
            @hybrid_property
            def full_name(self):
                return "instance"

            @full_name.expression
            def full_name(cls):
                return MockSqlExpr("first_name || ' ' || last_name")

        result = TestClass.full_name

        expect(isinstance(result, MockSqlExpr)).to_be_true()
        expect(result.expr).to_equal("first_name || ' ' || last_name")

    def test_class_access_returns_self_without_expression(self):
        """Test class access returns descriptor without expression."""
        class TestClass:
            @hybrid_property
            def full_name(self):
                return "instance"

        result = TestClass.full_name

        expect(isinstance(result, HybridPropertyDescriptor)).to_be_true()

    def test_setter_method_assigns_fset(self):
        """Test setter() method assigns the setter function."""
        descriptor = HybridPropertyDescriptor(lambda self: "value")

        def my_setter(self, value):
            self._value = value

        result = descriptor.setter(my_setter)

        expect(result).to_equal(descriptor)
        expect(descriptor.fset).to_equal(my_setter)

    def test_setter_allows_setting_value(self):
        """Test setter allows setting values on instance."""
        class TestClass:
            def __init__(self):
                self._name = "initial"

            @hybrid_property
            def name(self):
                return self._name

            @name.setter
            def name(self, value):
                self._name = value

        instance = TestClass()
        expect(instance.name).to_equal("initial")

        instance.name = "changed"
        expect(instance.name).to_equal("changed")

    def test_set_without_setter_raises_attribute_error(self):
        """Test setting without setter raises AttributeError."""
        class TestClass:
            @hybrid_property
            def readonly(self):
                return "value"

        instance = TestClass()

        with pytest.raises(AttributeError, match="can't set attribute 'readonly'"):
            instance.readonly = "new_value"

    def test_expression_method_assigns_expr(self):
        """Test expression() method assigns the expression function."""
        descriptor = HybridPropertyDescriptor(lambda self: "value")

        def my_expr(cls):
            return "SQL expression"

        result = descriptor.expression(my_expr)

        expect(result).to_equal(descriptor)
        expect(descriptor.expr).to_equal(my_expr)

    def test_repr(self):
        """Test __repr__ returns meaningful string."""
        def my_property(self):
            return "value"

        descriptor = HybridPropertyDescriptor(my_property)

        expect(repr(descriptor)).to_equal("<hybrid_property my_property>")


# HybridMethodDescriptor Tests

class TestHybridMethodDescriptor:
    """Test HybridMethodDescriptor class."""

    def test_descriptor_creation(self):
        """Test HybridMethodDescriptor can be created."""
        def method(self, arg):
            return arg

        descriptor = HybridMethodDescriptor(method)

        expect(descriptor.fget).to_equal(method)
        expect(descriptor.expr).to_be_none()
        expect(descriptor._name).to_be_none()

    def test_descriptor_copies_function_metadata(self):
        """Test descriptor copies __doc__ and __name__ from function."""
        def my_method(self, arg):
            """Method docstring."""
            return arg

        descriptor = HybridMethodDescriptor(my_method)

        expect(descriptor.__doc__).to_equal("Method docstring.")
        expect(descriptor.__name__).to_equal("my_method")

    def test_set_name_assigns_name(self):
        """Test __set_name__ assigns the attribute name."""
        def method(self, arg):
            return arg

        descriptor = HybridMethodDescriptor(method)
        descriptor.__set_name__(Mock, "is_older_than")

        expect(descriptor._name).to_equal("is_older_than")

    def test_instance_access_returns_callable(self):
        """Test instance access returns bound method."""
        class TestClass:
            def __init__(self):
                self.age = 30

            @hybrid_method
            def is_older_than(self, min_age):
                return self.age > min_age

        instance = TestClass()
        result = instance.is_older_than(25)

        expect(result).to_be_true()

    def test_instance_method_with_multiple_args(self):
        """Test instance method with multiple arguments."""
        class TestClass:
            def __init__(self):
                self.value = 100

            @hybrid_method
            def in_range(self, min_val, max_val):
                return min_val <= self.value <= max_val

        instance = TestClass()

        expect(instance.in_range(50, 150)).to_be_true()
        expect(instance.in_range(101, 200)).to_be_false()

    def test_class_access_returns_sql_expression(self):
        """Test class access returns SQL expression when defined."""
        class MockSqlExpr:
            def __init__(self, op, value):
                self.op = op
                self.value = value

        class TestClass:
            @hybrid_method
            def is_older_than(self, min_age):
                return self.age > min_age

            @is_older_than.expression
            def is_older_than(cls, min_age):
                return MockSqlExpr(">", min_age)

        result = TestClass.is_older_than(25)

        expect(isinstance(result, MockSqlExpr)).to_be_true()
        expect(result.op).to_equal(">")
        expect(result.value).to_equal(25)

    def test_class_access_without_expression(self):
        """Test class access without expression returns callable."""
        class TestClass:
            age = 30

            @hybrid_method
            def is_older_than(self, min_age):
                return self.age > min_age

        # Class access without expression should return callable
        result = TestClass.is_older_than(25)

        # Should call fget with the class
        expect(result).to_be_true()

    def test_expression_method_assigns_expr(self):
        """Test expression() method assigns the expression function."""
        descriptor = HybridMethodDescriptor(lambda self, x: x)

        def my_expr(cls, x):
            return f"SQL({x})"

        result = descriptor.expression(my_expr)

        expect(result).to_equal(descriptor)
        expect(descriptor.expr).to_equal(my_expr)

    def test_repr(self):
        """Test __repr__ returns meaningful string."""
        def my_method(self, arg):
            return arg

        descriptor = HybridMethodDescriptor(my_method)

        expect(repr(descriptor)).to_equal("<hybrid_method my_method>")


# ColumnPropertyDescriptor Tests

class TestColumnPropertyDescriptor:
    """Test ColumnPropertyDescriptor class."""

    def test_descriptor_creation(self):
        """Test ColumnPropertyDescriptor can be created."""
        descriptor = ColumnPropertyDescriptor("amount * (1 + tax_rate)")

        expect(descriptor.expression).to_equal("amount * (1 + tax_rate)")
        expect(descriptor._name).to_be_none()
        expect(descriptor._cache).to_equal({})

    def test_set_name_assigns_name(self):
        """Test __set_name__ assigns the attribute name."""
        descriptor = ColumnPropertyDescriptor("amount * 1.1")
        descriptor.__set_name__(Mock, "total")

        expect(descriptor._name).to_equal("total")

    def test_class_access_returns_descriptor(self):
        """Test class access returns the descriptor itself."""
        class TestClass:
            total = column_property("amount * 1.1")

        result = TestClass.total

        expect(isinstance(result, ColumnPropertyDescriptor)).to_be_true()

    def test_instance_access_gets_from_data(self):
        """Test instance access gets value from _data."""
        class TestClass:
            total = column_property("amount * 1.1")

        instance = TestClass()
        instance._data = {"total": 110.0}

        expect(instance.total).to_equal(110.0)

    def test_instance_access_returns_none_without_data(self):
        """Test instance access returns None without _data."""
        class TestClass:
            total = column_property("amount * 1.1")

        instance = TestClass()

        expect(instance.total).to_be_none()

    def test_instance_access_uses_cache(self):
        """Test instance access uses cache by instance id."""
        descriptor = ColumnPropertyDescriptor("amount * 1.1")
        descriptor._name = "total"

        instance = Mock()
        instance._data = {}

        # Manually set cache
        descriptor._cache[id(instance)] = 150.0

        result = descriptor.__get__(instance, type(instance))

        expect(result).to_equal(150.0)

    def test_set_raises_attribute_error(self):
        """Test setting raises AttributeError (read-only)."""
        class TestClass:
            total = column_property("amount * 1.1")

        instance = TestClass()

        with pytest.raises(AttributeError, match="can't set attribute 'total': column_property is read-only"):
            instance.total = 100.0

    def test_repr(self):
        """Test __repr__ returns meaningful string."""
        descriptor = ColumnPropertyDescriptor("amount * (1 + tax_rate)")

        expect(repr(descriptor)).to_equal("<column_property 'amount * (1 + tax_rate)'>")


# ComputedColumn Tests

class TestComputedColumn:
    """Test ComputedColumn class."""

    def test_computed_column_creation_stored(self):
        """Test ComputedColumn can be created with stored=True."""
        computed = ComputedColumn("price * quantity", stored=True)

        expect(computed.expression).to_equal("price * quantity")
        expect(computed.stored).to_be_true()
        expect(computed._name).to_be_none()

    def test_computed_column_creation_virtual(self):
        """Test ComputedColumn can be created with stored=False."""
        computed = ComputedColumn("price * 0.1", stored=False)

        expect(computed.expression).to_equal("price * 0.1")
        expect(computed.stored).to_be_false()

    def test_set_name_assigns_name(self):
        """Test __set_name__ assigns the attribute name."""
        computed = ComputedColumn("price * quantity")
        computed.__set_name__(Mock, "total")

        expect(computed._name).to_equal("total")

    def test_class_access_returns_descriptor(self):
        """Test class access returns the descriptor itself."""
        class TestClass:
            total = Computed("price * quantity")

        result = TestClass.total

        expect(isinstance(result, ComputedColumn)).to_be_true()

    def test_instance_access_gets_from_data(self):
        """Test instance access gets value from _data."""
        class TestClass:
            total = Computed("price * quantity")

        instance = TestClass()
        instance._data = {"total": 1500.0}

        expect(instance.total).to_equal(1500.0)

    def test_instance_access_returns_none_without_data(self):
        """Test instance access returns None without _data."""
        class TestClass:
            total = Computed("price * quantity")

        instance = TestClass()

        expect(instance.total).to_be_none()

    def test_set_raises_attribute_error(self):
        """Test setting raises AttributeError (read-only)."""
        class TestClass:
            total = Computed("price * quantity")

        instance = TestClass()

        with pytest.raises(AttributeError, match="can't set attribute 'total': computed columns are read-only"):
            instance.total = 1000.0

    def test_to_sql_stored(self):
        """Test to_sql() generates correct DDL for STORED column."""
        computed = ComputedColumn("price * quantity", stored=True)

        sql = computed.to_sql("FLOAT")

        expect(sql).to_equal("FLOAT GENERATED ALWAYS AS (price * quantity) STORED")

    def test_to_sql_virtual(self):
        """Test to_sql() generates correct DDL for virtual column."""
        computed = ComputedColumn("price * 0.1", stored=False)

        sql = computed.to_sql("DECIMAL(10,2)")

        expect(sql).to_equal("DECIMAL(10,2) GENERATED ALWAYS AS (price * 0.1)")

    def test_to_sql_custom_type(self):
        """Test to_sql() works with different data types."""
        computed = ComputedColumn("quantity * 2", stored=True)

        sql = computed.to_sql("INTEGER")

        expect(sql).to_equal("INTEGER GENERATED ALWAYS AS (quantity * 2) STORED")

    def test_repr_stored(self):
        """Test __repr__ for stored computed column."""
        computed = ComputedColumn("price * quantity", stored=True)

        expect(repr(computed)).to_equal("<Computed 'price * quantity' (stored)>")

    def test_repr_virtual(self):
        """Test __repr__ for virtual computed column."""
        computed = ComputedColumn("price * 0.1", stored=False)

        expect(repr(computed)).to_equal("<Computed 'price * 0.1' (virtual)>")


# hybrid_property() function tests

class TestHybridPropertyFunction:
    """Test hybrid_property() decorator function."""

    def test_hybrid_property_returns_descriptor(self):
        """Test hybrid_property() returns HybridPropertyDescriptor."""
        def my_property(self):
            return "value"

        result = hybrid_property(my_property)

        expect(isinstance(result, HybridPropertyDescriptor)).to_be_true()

    def test_hybrid_property_as_decorator(self):
        """Test hybrid_property used as decorator."""
        class TestClass:
            def __init__(self):
                self.value = 42

            @hybrid_property
            def my_prop(self):
                return self.value * 2

        instance = TestClass()

        expect(instance.my_prop).to_equal(84)

    def test_hybrid_property_with_expression(self):
        """Test hybrid_property with SQL expression."""
        class TestClass:
            def __init__(self):
                self.first_name = "John"
                self.last_name = "Doe"

            @hybrid_property
            def full_name(self):
                return f"{self.first_name} {self.last_name}"

            @full_name.expression
            def full_name(cls):
                return "SQL_CONCAT"

        instance = TestClass()

        expect(instance.full_name).to_equal("John Doe")
        expect(TestClass.full_name).to_equal("SQL_CONCAT")


# hybrid_method() function tests

class TestHybridMethodFunction:
    """Test hybrid_method() decorator function."""

    def test_hybrid_method_returns_descriptor(self):
        """Test hybrid_method() returns HybridMethodDescriptor."""
        def my_method(self, arg):
            return arg

        result = hybrid_method(my_method)

        expect(isinstance(result, HybridMethodDescriptor)).to_be_true()

    def test_hybrid_method_as_decorator(self):
        """Test hybrid_method used as decorator."""
        class TestClass:
            def __init__(self):
                self.age = 30

            @hybrid_method
            def is_older_than(self, min_age):
                return self.age > min_age

        instance = TestClass()

        expect(instance.is_older_than(25)).to_be_true()
        expect(instance.is_older_than(35)).to_be_false()

    def test_hybrid_method_with_expression(self):
        """Test hybrid_method with SQL expression."""
        class TestClass:
            def __init__(self):
                self.age = 30

            @hybrid_method
            def is_older_than(self, min_age):
                return self.age > min_age

            @is_older_than.expression
            def is_older_than(cls, min_age):
                return f"age > {min_age}"

        instance = TestClass()

        expect(instance.is_older_than(25)).to_be_true()
        expect(TestClass.is_older_than(25)).to_equal("age > 25")


# column_property() function tests

class TestColumnPropertyFunction:
    """Test column_property() factory function."""

    def test_column_property_returns_descriptor(self):
        """Test column_property() returns ColumnPropertyDescriptor."""
        result = column_property("amount * 1.1")

        expect(isinstance(result, ColumnPropertyDescriptor)).to_be_true()

    def test_column_property_stores_expression(self):
        """Test column_property() stores the SQL expression."""
        result = column_property("amount * (1 + tax_rate)")

        expect(result.expression).to_equal("amount * (1 + tax_rate)")

    def test_column_property_in_class(self):
        """Test column_property() used in class definition."""
        class TestClass:
            amount = 100.0
            tax_rate = 0.2
            total = column_property("amount * (1 + tax_rate)")

        instance = TestClass()
        instance._data = {"total": 120.0}

        expect(instance.total).to_equal(120.0)


# default_factory() function tests

class TestDefaultFactoryFunction:
    """Test default_factory() function."""

    def test_default_factory_returns_callable(self):
        """Test default_factory() returns the factory callable."""
        def factory():
            return "default"

        result = default_factory(factory)

        expect(result).to_equal(factory)

    def test_default_factory_with_lambda(self):
        """Test default_factory() with lambda function."""
        factory = default_factory(lambda: 42)

        expect(factory()).to_equal(42)

    def test_default_factory_called_each_time(self):
        """Test default_factory callable returns new value each call."""
        counter = [0]

        def factory():
            counter[0] += 1
            return counter[0]

        my_factory = default_factory(factory)

        expect(my_factory()).to_equal(1)
        expect(my_factory()).to_equal(2)
        expect(my_factory()).to_equal(3)

    def test_default_factory_with_datetime(self):
        """Test default_factory() with datetime.utcnow."""
        from datetime import datetime
        from unittest.mock import Mock

        mock_utcnow = Mock(return_value=datetime(2024, 1, 1, 12, 0, 0))
        factory = default_factory(mock_utcnow)

        result = factory()

        expect(result).to_equal(datetime(2024, 1, 1, 12, 0, 0))
        mock_utcnow.assert_called_once()

    def test_default_factory_with_uuid(self):
        """Test default_factory() with UUID generation."""
        import uuid

        factory = default_factory(lambda: str(uuid.uuid4()))

        value1 = factory()
        value2 = factory()

        # Each call should generate different UUID
        expect(value1 != value2).to_be_true()
        expect(len(value1)).to_equal(36)  # UUID string length


# Integration Tests

class TestComputedAttributesIntegration:
    """Test computed attributes with Table classes."""

    def test_hybrid_property_on_table(self, sample_table_class):
        """Test hybrid_property works with Table class."""
        # Add hybrid property to class
        @hybrid_property
        def full_name(self):
            return f"{self.first_name} {self.last_name}"

        sample_table_class.full_name = full_name

        user = sample_table_class(first_name="John", last_name="Doe")

        expect(user.full_name).to_equal("John Doe")

    def test_hybrid_method_on_table(self, sample_table_class):
        """Test hybrid_method works with Table class."""
        # Add hybrid method to class
        @hybrid_method
        def is_adult(self, min_age=18):
            return self.age >= min_age

        sample_table_class.is_adult = is_adult

        user1 = sample_table_class(first_name="John", last_name="Doe", age=25)
        user2 = sample_table_class(first_name="Jane", last_name="Smith", age=15)

        expect(user1.is_adult()).to_be_true()
        expect(user2.is_adult()).to_be_false()
        expect(user2.is_adult(min_age=10)).to_be_true()

    def test_computed_on_table(self):
        """Test Computed column works with Table class."""
        # Define class with computed column
        class Product(Table):
            price: float
            quantity: int
            total = Computed("price * quantity", stored=True)

            class Settings:
                table_name = "products"

        product = Product(price=10.0, quantity=5)
        product._data = {"total": 50.0}

        expect(product.total).to_equal(50.0)

    def test_column_property_on_table(self):
        """Test column_property works with Table class."""
        # Define class with column property
        class Product(Table):
            price: float
            quantity: int
            discounted = column_property("price * 0.9")

            class Settings:
                table_name = "products"

        product = Product(price=100.0, quantity=2)
        product._data = {"discounted": 90.0}

        expect(product.discounted).to_equal(90.0)

    def test_multiple_computed_attributes(self, sample_table_class):
        """Test multiple computed attributes on same class."""
        # Add multiple hybrid properties
        @hybrid_property
        def full_name(self):
            return f"{self.first_name} {self.last_name}"

        @hybrid_property
        def initials(self):
            return f"{self.first_name[0]}.{self.last_name[0]}."

        @hybrid_method
        def is_older_than(self, age):
            return self.age > age

        sample_table_class.full_name = full_name
        sample_table_class.initials = initials
        sample_table_class.is_older_than = is_older_than

        user = sample_table_class(first_name="John", last_name="Doe", age=30)

        expect(user.full_name).to_equal("John Doe")
        expect(user.initials).to_equal("J.D.")
        expect(user.is_older_than(25)).to_be_true()


# Edge Cases and Error Handling

class TestComputedAttributesEdgeCases:
    """Test edge cases and error handling."""

    def test_hybrid_property_with_none_value(self):
        """Test hybrid_property handles None values."""
        class TestClass:
            def __init__(self):
                self.value = None

            @hybrid_property
            def computed(self):
                return self.value if self.value else "default"

        instance = TestClass()

        expect(instance.computed).to_equal("default")

    def test_hybrid_method_with_no_args(self):
        """Test hybrid_method with no additional arguments."""
        class TestClass:
            def __init__(self):
                self.ready = True

            @hybrid_method
            def is_ready(self):
                return self.ready

        instance = TestClass()

        expect(instance.is_ready()).to_be_true()

    def test_computed_column_with_complex_expression(self):
        """Test ComputedColumn with complex SQL expression."""
        computed = ComputedColumn(
            "CASE WHEN quantity > 10 THEN price * 0.9 ELSE price END",
            stored=True
        )

        sql = computed.to_sql("DECIMAL(10,2)")

        expect("CASE WHEN quantity > 10" in sql).to_be_true()
        expect("STORED" in sql).to_be_true()

    def test_column_property_without_table_data(self):
        """Test column_property gracefully handles missing _data."""
        class TestClass:
            total = column_property("amount * 1.1")

        instance = TestClass()
        # No _data attribute

        expect(instance.total).to_be_none()

    def test_computed_alias(self):
        """Test Computed is alias for ComputedColumn."""
        expect(Computed).to_equal(ComputedColumn)

        computed1 = Computed("price * quantity")
        computed2 = ComputedColumn("price * quantity")

        expect(type(computed1)).to_equal(type(computed2))

    def test_hybrid_property_setter_with_validation(self):
        """Test hybrid_property setter with validation logic."""
        class TestClass:
            def __init__(self):
                self._age = 0

            @hybrid_property
            def age(self):
                return self._age

            @age.setter
            def age(self, value):
                if value < 0:
                    raise ValueError("Age cannot be negative")
                self._age = value

        instance = TestClass()
        instance.age = 25
        expect(instance.age).to_equal(25)

        with pytest.raises(ValueError, match="Age cannot be negative"):
            instance.age = -5

    def test_hybrid_method_with_kwargs(self):
        """Test hybrid_method with keyword arguments."""
        class TestClass:
            def __init__(self):
                self.value = 50

            @hybrid_method
            def in_range(self, min_val=0, max_val=100):
                return min_val <= self.value <= max_val

        instance = TestClass()

        expect(instance.in_range()).to_be_true()
        expect(instance.in_range(min_val=60)).to_be_false()
        expect(instance.in_range(max_val=40)).to_be_false()
        expect(instance.in_range(min_val=40, max_val=60)).to_be_true()


# Documentation Examples Tests

class TestDocumentationExamples:
    """Test examples from module docstring work correctly."""

    def test_hybrid_property_example(self):
        """Test the hybrid_property example from docstring."""
        class User(Table):
            first_name: str
            last_name: str

            class Settings:
                table_name = "users"

            @hybrid_property
            def full_name(self):
                return f"{self.first_name} {self.last_name}"

        user = User(first_name="Alice", last_name="Smith")

        expect(user.full_name).to_equal("Alice Smith")

    def test_computed_column_example(self):
        """Test the Computed column example from docstring."""
        class Product(Table):
            price: float
            quantity: int
            total = Computed("price * quantity", stored=True)

            class Settings:
                table_name = "products"

        product = Product(price=10.0, quantity=5)
        product._data = {"total": 50.0}

        expect(product.total).to_equal(50.0)

        # Check DDL generation
        sql = Product.total.to_sql("FLOAT")
        expect("GENERATED ALWAYS AS (price * quantity) STORED" in sql).to_be_true()

    def test_hybrid_method_example(self):
        """Test the hybrid_method example from docstring."""
        class User(Table):
            age: int

            class Settings:
                table_name = "users"

            @hybrid_method
            def is_older_than(self, min_age):
                return self.age > min_age

        user = User(age=30)

        expect(user.is_older_than(25)).to_be_true()
        expect(user.is_older_than(35)).to_be_false()

    def test_column_property_example(self):
        """Test the column_property example from docstring."""
        class Order(Table):
            amount: float
            tax_rate: float
            total = column_property("amount * (1 + tax_rate)")

            class Settings:
                table_name = "orders"

        order = Order(amount=100.0, tax_rate=0.2)
        order._data = {"total": 120.0}

        expect(order.total).to_equal(120.0)
