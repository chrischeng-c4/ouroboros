"""
Unit tests for inheritance patterns module.

Tests InheritanceType, InheritanceConfig, inheritance decorator,
SingleTableInheritance, JoinedTableInheritance, ConcreteTableInheritance,
PolymorphicQueryMixin, and helper functions.
"""
import pytest
from typing import Optional
from ouroboros.test import expect
from ouroboros.postgres import Table
from ouroboros.postgres.inheritance import (
    InheritanceType,
    InheritanceConfig,
    inheritance,
    SingleTableInheritance,
    JoinedTableInheritance,
    ConcreteTableInheritance,
    PolymorphicQueryMixin,
    get_inheritance_type,
    get_discriminator_column,
    get_discriminator_value,
    register_polymorphic_class,
    get_polymorphic_map,
    _polymorphic_registry,
)


class TestInheritanceType:
    """Test InheritanceType enum."""

    def test_single_table_exists(self):
        """Test SINGLE_TABLE enum value exists."""
        expect(InheritanceType.SINGLE_TABLE.value).to_equal("single_table")

    def test_joined_exists(self):
        """Test JOINED enum value exists."""
        expect(InheritanceType.JOINED.value).to_equal("joined")

    def test_concrete_exists(self):
        """Test CONCRETE enum value exists."""
        expect(InheritanceType.CONCRETE.value).to_equal("concrete")

    def test_all_values_present(self):
        """Test all three inheritance types are defined."""
        values = [e.value for e in InheritanceType]
        expect(len(values)).to_equal(3)
        expect("single_table" in values).to_be_true()
        expect("joined" in values).to_be_true()
        expect("concrete" in values).to_be_true()


class TestInheritanceConfig:
    """Test InheritanceConfig dataclass."""

    def test_config_creation_minimal(self):
        """Test creating config with minimal parameters."""
        config = InheritanceConfig(
            inheritance_type=InheritanceType.SINGLE_TABLE
        )

        expect(config.inheritance_type).to_equal(InheritanceType.SINGLE_TABLE)
        expect(config.discriminator_column).to_equal("type")
        expect(config.discriminator_value).to_be_none()
        expect(config.polymorphic_on).to_equal("type")

    def test_config_creation_all_fields(self):
        """Test creating config with all fields specified."""
        config = InheritanceConfig(
            inheritance_type=InheritanceType.SINGLE_TABLE,
            discriminator_column="employee_type",
            discriminator_value="manager",
            polymorphic_on="employee_type",
        )

        expect(config.inheritance_type).to_equal(InheritanceType.SINGLE_TABLE)
        expect(config.discriminator_column).to_equal("employee_type")
        expect(config.discriminator_value).to_equal("manager")
        expect(config.polymorphic_on).to_equal("employee_type")

    def test_config_polymorphic_on_defaults_to_discriminator(self):
        """Test polymorphic_on defaults to discriminator_column if not set."""
        config = InheritanceConfig(
            inheritance_type=InheritanceType.SINGLE_TABLE,
            discriminator_column="emp_type",
        )

        expect(config.polymorphic_on).to_equal("emp_type")

    def test_config_polymorphic_on_explicit(self):
        """Test polymorphic_on can be explicitly different from discriminator."""
        config = InheritanceConfig(
            inheritance_type=InheritanceType.SINGLE_TABLE,
            discriminator_column="type",
            polymorphic_on="kind",
        )

        expect(config.discriminator_column).to_equal("type")
        expect(config.polymorphic_on).to_equal("kind")

    def test_config_with_joined_type(self):
        """Test config with JOINED inheritance type."""
        config = InheritanceConfig(
            inheritance_type=InheritanceType.JOINED
        )

        expect(config.inheritance_type).to_equal(InheritanceType.JOINED)

    def test_config_with_concrete_type(self):
        """Test config with CONCRETE inheritance type."""
        config = InheritanceConfig(
            inheritance_type=InheritanceType.CONCRETE
        )

        expect(config.inheritance_type).to_equal(InheritanceType.CONCRETE)


class TestInheritanceDecorator:
    """Test @inheritance decorator."""

    def test_decorator_applies_config(self):
        """Test decorator applies inheritance config to class."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table):
            name: str
            type: str

        expect(hasattr(Employee, "_inheritance_config")).to_be_true()
        expect(Employee._inheritance_config.inheritance_type).to_equal(
            InheritanceType.SINGLE_TABLE
        )
        expect(Employee._inheritance_config.discriminator_column).to_equal("type")

    def test_decorator_default_discriminator(self):
        """Test decorator uses default discriminator 'type'."""

        @inheritance(type=InheritanceType.SINGLE_TABLE)
        class Vehicle(Table):
            make: str

        expect(Vehicle._inheritance_config.discriminator_column).to_equal("type")

    def test_decorator_custom_discriminator(self):
        """Test decorator with custom discriminator column."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="vehicle_type")
        class Vehicle(Table):
            make: str
            vehicle_type: str

        expect(Vehicle._inheritance_config.discriminator_column).to_equal("vehicle_type")

    def test_decorator_polymorphic_on(self):
        """Test decorator with polymorphic_on parameter."""

        @inheritance(
            type=InheritanceType.SINGLE_TABLE,
            discriminator="type",
            polymorphic_on="kind"
        )
        class Animal(Table):
            name: str

        expect(Animal._inheritance_config.discriminator_column).to_equal("type")
        expect(Animal._inheritance_config.polymorphic_on).to_equal("kind")

    def test_decorator_joined_inheritance(self):
        """Test decorator with JOINED inheritance."""

        @inheritance(type=InheritanceType.JOINED)
        class Person(Table):
            name: str

        expect(Person._inheritance_config.inheritance_type).to_equal(
            InheritanceType.JOINED
        )

    def test_decorator_concrete_inheritance(self):
        """Test decorator with CONCRETE inheritance."""

        @inheritance(type=InheritanceType.CONCRETE)
        class Document(Table):
            title: str

        expect(Document._inheritance_config.inheritance_type).to_equal(
            InheritanceType.CONCRETE
        )

    def test_decorator_initializes_registry(self):
        """Test decorator initializes polymorphic registry for base class."""

        @inheritance(type=InheritanceType.SINGLE_TABLE)
        class Product(Table):
            name: str

        expect(Product in _polymorphic_registry).to_be_true()
        expect(isinstance(_polymorphic_registry[Product], dict)).to_be_true()


class TestSingleTableInheritance:
    """Test SingleTableInheritance mixin."""

    def test_discriminator_value_attribute(self):
        """Test __discriminator_value__ class attribute exists."""

        class Employee(Table, SingleTableInheritance):
            name: str

        expect(hasattr(Employee, "__discriminator_value__")).to_be_true()
        expect(Employee.__discriminator_value__).to_be_none()

    def test_inheritance_config_attribute(self):
        """Test _inheritance_config class attribute exists."""

        @inheritance(type=InheritanceType.SINGLE_TABLE)
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        expect(hasattr(Employee, "_inheritance_config")).to_be_true()
        expect(isinstance(Employee._inheritance_config, InheritanceConfig)).to_be_true()

    def test_subclass_registration(self):
        """Test subclass registers in polymorphic registry."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        expect(Employee in _polymorphic_registry).to_be_true()
        expect("manager" in _polymorphic_registry[Employee]).to_be_true()
        expect(_polymorphic_registry[Employee]["manager"]).to_equal(Manager)

    def test_multiple_subclass_registration(self):
        """Test multiple subclasses register correctly."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        class Engineer(Employee):
            __discriminator_value__ = "engineer"
            programming_language: str

        expect(len(_polymorphic_registry[Employee])).to_equal(2)
        expect(_polymorphic_registry[Employee]["manager"]).to_equal(Manager)
        expect(_polymorphic_registry[Employee]["engineer"]).to_equal(Engineer)

    def test_get_discriminator_filter_base_class(self):
        """Test _get_discriminator_filter returns None for base class."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        result = Employee._get_discriminator_filter()
        expect(result).to_be_none()

    def test_get_discriminator_filter_subclass(self):
        """Test _get_discriminator_filter returns filter for subclass."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="emp_type")
        class Employee(Table, SingleTableInheritance):
            name: str
            emp_type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        result = Manager._get_discriminator_filter()
        expect(result is not None).to_be_true()
        expect(result[0]).to_equal("emp_type")
        expect(result[1]).to_equal("manager")

    def test_get_discriminator_filter_no_value(self):
        """Test _get_discriminator_filter returns None if no discriminator value."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        class Subclass(Employee):
            # No __discriminator_value__ set
            pass

        result = Subclass._get_discriminator_filter()
        expect(result).to_be_none()

    def test_polymorphic_identity_base_class(self):
        """Test polymorphic_identity returns None for base class."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        expect(Employee.polymorphic_identity()).to_be_none()

    def test_polymorphic_identity_subclass(self):
        """Test polymorphic_identity returns discriminator value for subclass."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        expect(Manager.polymorphic_identity()).to_equal("manager")


class TestJoinedTableInheritance:
    """Test JoinedTableInheritance mixin."""

    def test_parent_table_attribute(self):
        """Test _parent_table class attribute exists."""

        class Employee(Table, JoinedTableInheritance):
            name: str

            class Settings:
                table_name = "employees"

        expect(hasattr(Employee, "_parent_table")).to_be_true()

    def test_subclass_tracks_parent_table(self):
        """Test subclass identifies parent table."""

        class Employee(Table, JoinedTableInheritance):
            name: str

            class Settings:
                table_name = "employees"

        class Manager(Employee):
            department: str

            class Settings:
                table_name = "managers"

        expect(Manager._parent_table is not None).to_be_true()
        expect(Manager._parent_table).to_equal(Employee)

    def test_get_join_config_base_class(self):
        """Test _get_join_config returns None for base class."""

        @inheritance(type=InheritanceType.JOINED)
        class Employee(Table, JoinedTableInheritance):
            name: str

            class Settings:
                table_name = "employees"

        result = Employee._get_join_config()
        expect(result).to_be_none()

    def test_get_join_config_subclass(self):
        """Test _get_join_config returns config for subclass."""

        @inheritance(type=InheritanceType.JOINED)
        class Employee(Table, JoinedTableInheritance):
            name: str

            class Settings:
                table_name = "employees"

        class Manager(Employee):
            department: str

            class Settings:
                table_name = "managers"

        result = Manager._get_join_config()
        expect(result is not None).to_be_true()
        expect(result["parent_table"]).to_equal("employees")
        expect(result["parent_pk"]).to_equal("id")
        expect(result["child_table"]).to_equal("managers")
        expect(result["child_fk"]).to_equal("id")

    def test_subclass_registration(self):
        """Test subclass properly extends parent class."""

        @inheritance(type=InheritanceType.JOINED)
        class Vehicle(Table, JoinedTableInheritance):
            make: str
            model: str

            class Settings:
                table_name = "vehicles"

        class Car(Vehicle):
            num_doors: int

            class Settings:
                table_name = "cars"

        class Truck(Vehicle):
            bed_length: float

            class Settings:
                table_name = "trucks"

        expect(Car._parent_table).to_equal(Vehicle)
        expect(Truck._parent_table).to_equal(Vehicle)
        expect(issubclass(Car, Vehicle)).to_be_true()
        expect(issubclass(Truck, Vehicle)).to_be_true()


class TestConcreteTableInheritance:
    """Test ConcreteTableInheritance mixin."""

    def test_concrete_subclasses_attribute(self):
        """Test _concrete_subclasses class attribute exists."""

        class Employee(Table, ConcreteTableInheritance):
            name: str

        expect(hasattr(Employee, "_concrete_subclasses")).to_be_true()
        expect(isinstance(Employee._concrete_subclasses, list)).to_be_true()

    def test_subclass_registration(self):
        """Test subclass registers in concrete subclasses list."""

        @inheritance(type=InheritanceType.CONCRETE)
        class Employee(Table, ConcreteTableInheritance):
            name: str
            email: str

            class Settings:
                table_name = "employees"

        class Manager(Employee):
            name: str
            email: str
            department: str

            class Settings:
                table_name = "managers"

        # Find the base that has _concrete_subclasses
        for base in Manager.__mro__[1:]:
            if hasattr(base, "_concrete_subclasses") and isinstance(base, type):
                if issubclass(base, ConcreteTableInheritance):
                    expect(Manager in base._concrete_subclasses).to_be_true()
                    break

    def test_multiple_subclass_registration(self):
        """Test multiple subclasses register independently."""

        @inheritance(type=InheritanceType.CONCRETE)
        class Employee(Table, ConcreteTableInheritance):
            name: str
            email: str

            class Settings:
                table_name = "employees"

        class Manager(Employee):
            name: str
            email: str
            department: str

            class Settings:
                table_name = "managers"

        class Engineer(Employee):
            name: str
            email: str
            programming_language: str

            class Settings:
                table_name = "engineers"

        # Find base with _concrete_subclasses
        for base in Manager.__mro__[1:]:
            if hasattr(base, "_concrete_subclasses") and isinstance(base, type):
                if issubclass(base, ConcreteTableInheritance):
                    expect(Manager in base._concrete_subclasses).to_be_true()
                    expect(Engineer in base._concrete_subclasses).to_be_true()
                    break

    def test_independent_tables(self):
        """Test each concrete subclass has independent table."""

        @inheritance(type=InheritanceType.CONCRETE)
        class Document(Table, ConcreteTableInheritance):
            title: str
            content: str

            class Settings:
                table_name = "documents"

        class Article(Document):
            title: str
            content: str
            author: str

            class Settings:
                table_name = "articles"

        class Report(Document):
            title: str
            content: str
            department: str

            class Settings:
                table_name = "reports"

        expect(Document._table_name).to_equal("documents")
        expect(Article._table_name).to_equal("articles")
        expect(Report._table_name).to_equal("reports")


class TestPolymorphicQueryMixin:
    """Test PolymorphicQueryMixin."""

    def test_polymorphic_identity_with_value(self):
        """Test polymorphic_identity returns discriminator value."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, PolymorphicQueryMixin):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        expect(Manager.polymorphic_identity()).to_equal("manager")

    def test_polymorphic_identity_without_value(self):
        """Test polymorphic_identity returns None if no discriminator value."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, PolymorphicQueryMixin):
            name: str
            type: str

        expect(Employee.polymorphic_identity()).to_be_none()

    def test_get_subclasses(self):
        """Test get_subclasses returns registered subclasses."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance, PolymorphicQueryMixin):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        class Engineer(Employee):
            __discriminator_value__ = "engineer"
            programming_language: str

        subclasses = Employee.get_subclasses()
        expect(len(subclasses)).to_equal(2)
        expect(Manager in subclasses).to_be_true()
        expect(Engineer in subclasses).to_be_true()

    def test_get_subclasses_empty(self):
        """Test get_subclasses returns empty list if no subclasses."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Product(Table, PolymorphicQueryMixin):
            name: str
            type: str

        subclasses = Product.get_subclasses()
        expect(len(subclasses)).to_equal(0)

    @pytest.mark.asyncio
    async def test_fetch_polymorphic_not_implemented(self):
        """Test fetch_polymorphic raises NotImplementedError."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, PolymorphicQueryMixin):
            name: str
            type: str

        exc_info = expect(lambda: await Employee.fetch_polymorphic()).to_raise(NotImplementedError)

        expect("Polymorphic queries require integration" in str(exc_info.value)).to_be_true()


class TestHelperFunctions:
    """Test helper functions."""

    def test_get_inheritance_type_with_config(self):
        """Test get_inheritance_type returns type from config."""

        @inheritance(type=InheritanceType.SINGLE_TABLE)
        class Employee(Table):
            name: str

        result = get_inheritance_type(Employee)
        expect(result).to_equal(InheritanceType.SINGLE_TABLE)

    def test_get_inheritance_type_from_parent(self):
        """Test get_inheritance_type finds config in parent class."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        result = get_inheritance_type(Manager)
        expect(result).to_equal(InheritanceType.SINGLE_TABLE)

    def test_get_inheritance_type_no_config(self):
        """Test get_inheritance_type returns None if no config."""

        class Product(Table):
            name: str

        result = get_inheritance_type(Product)
        expect(result).to_be_none()

    def test_get_discriminator_column_with_config(self):
        """Test get_discriminator_column returns column name."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="emp_type")
        class Employee(Table):
            name: str
            emp_type: str

        result = get_discriminator_column(Employee)
        expect(result).to_equal("emp_type")

    def test_get_discriminator_column_from_parent(self):
        """Test get_discriminator_column finds column in parent class."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        result = get_discriminator_column(Manager)
        expect(result).to_equal("type")

    def test_get_discriminator_column_no_config(self):
        """Test get_discriminator_column returns None if no config."""

        class Product(Table):
            name: str

        result = get_discriminator_column(Product)
        expect(result).to_be_none()

    def test_get_discriminator_value_with_value(self):
        """Test get_discriminator_value returns value."""

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        result = get_discriminator_value(Manager)
        expect(result).to_equal("manager")

    def test_get_discriminator_value_no_value(self):
        """Test get_discriminator_value returns None if no value."""

        class Employee(Table):
            name: str

        result = get_discriminator_value(Employee)
        expect(result).to_be_none()

    def test_register_polymorphic_class(self):
        """Test register_polymorphic_class adds to registry."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        register_polymorphic_class(Employee, Manager, "manager")

        expect(Employee in _polymorphic_registry).to_be_true()
        expect("manager" in _polymorphic_registry[Employee]).to_be_true()
        expect(_polymorphic_registry[Employee]["manager"]).to_equal(Manager)

    def test_register_polymorphic_class_initializes_registry(self):
        """Test register_polymorphic_class creates registry entry if needed."""
        # Clear registry
        _polymorphic_registry.clear()

        class Vehicle(Table):
            make: str

        class Car(Vehicle):
            num_doors: int

        register_polymorphic_class(Vehicle, Car, "car")

        expect(Vehicle in _polymorphic_registry).to_be_true()
        expect(_polymorphic_registry[Vehicle]["car"]).to_equal(Car)

    def test_register_polymorphic_class_multiple_children(self):
        """Test register_polymorphic_class handles multiple children."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Animal(Table):
            name: str
            type: str

        class Dog(Animal):
            breed: str

        class Cat(Animal):
            indoor: bool

        register_polymorphic_class(Animal, Dog, "dog")
        register_polymorphic_class(Animal, Cat, "cat")

        expect(len(_polymorphic_registry[Animal])).to_equal(2)
        expect(_polymorphic_registry[Animal]["dog"]).to_equal(Dog)
        expect(_polymorphic_registry[Animal]["cat"]).to_equal(Cat)

    def test_get_polymorphic_map_with_entries(self):
        """Test get_polymorphic_map returns mapping."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance):
            name: str
            type: str

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str

        class Engineer(Employee):
            __discriminator_value__ = "engineer"
            programming_language: str

        result = get_polymorphic_map(Employee)

        expect(isinstance(result, dict)).to_be_true()
        expect(len(result)).to_equal(2)
        expect(result["manager"]).to_equal(Manager)
        expect(result["engineer"]).to_equal(Engineer)

    def test_get_polymorphic_map_empty(self):
        """Test get_polymorphic_map returns empty dict if no entries."""
        # Clear registry
        _polymorphic_registry.clear()

        class Product(Table):
            name: str

        result = get_polymorphic_map(Product)

        expect(isinstance(result, dict)).to_be_true()
        expect(len(result)).to_equal(0)

    def test_get_polymorphic_map_not_registered(self):
        """Test get_polymorphic_map returns empty dict if class not in registry."""
        class UnregisteredClass(Table):
            name: str

        result = get_polymorphic_map(UnregisteredClass)

        expect(isinstance(result, dict)).to_be_true()
        expect(len(result)).to_equal(0)


class TestIntegrationScenarios:
    """Test realistic integration scenarios."""

    def test_single_table_complete_hierarchy(self):
        """Test complete single table inheritance hierarchy."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
        class Employee(Table, SingleTableInheritance, PolymorphicQueryMixin):
            name: str
            email: str
            type: str

            class Settings:
                table_name = "employees"

        class Manager(Employee):
            __discriminator_value__ = "manager"
            department: str
            num_reports: int

        class Engineer(Employee):
            __discriminator_value__ = "engineer"
            programming_language: str
            years_experience: int

        class Contractor(Employee):
            __discriminator_value__ = "contractor"
            agency: str
            contract_end_date: str

        # Verify inheritance config
        expect(get_inheritance_type(Employee)).to_equal(InheritanceType.SINGLE_TABLE)
        expect(get_discriminator_column(Employee)).to_equal("type")

        # Verify discriminator values
        expect(get_discriminator_value(Manager)).to_equal("manager")
        expect(get_discriminator_value(Engineer)).to_equal("engineer")
        expect(get_discriminator_value(Contractor)).to_equal("contractor")

        # Verify registry
        poly_map = get_polymorphic_map(Employee)
        expect(len(poly_map)).to_equal(3)
        expect(poly_map["manager"]).to_equal(Manager)
        expect(poly_map["engineer"]).to_equal(Engineer)
        expect(poly_map["contractor"]).to_equal(Contractor)

        # Verify subclasses
        subclasses = Employee.get_subclasses()
        expect(len(subclasses)).to_equal(3)

    def test_joined_table_hierarchy(self):
        """Test complete joined table inheritance hierarchy."""

        @inheritance(type=InheritanceType.JOINED)
        class Vehicle(Table, JoinedTableInheritance):
            make: str
            model: str
            year: int

            class Settings:
                table_name = "vehicles"

        class Car(Vehicle):
            num_doors: int
            trunk_size: float

            class Settings:
                table_name = "cars"

        class Truck(Vehicle):
            bed_length: float
            towing_capacity: int

            class Settings:
                table_name = "trucks"

        # Verify inheritance type
        expect(get_inheritance_type(Vehicle)).to_equal(InheritanceType.JOINED)

        # Verify parent tracking
        expect(Car._parent_table).to_equal(Vehicle)
        expect(Truck._parent_table).to_equal(Vehicle)

        # Verify join config
        car_config = Car._get_join_config()
        expect(car_config["parent_table"]).to_equal("vehicles")
        expect(car_config["child_table"]).to_equal("cars")

        truck_config = Truck._get_join_config()
        expect(truck_config["parent_table"]).to_equal("vehicles")
        expect(truck_config["child_table"]).to_equal("trucks")

    def test_concrete_table_hierarchy(self):
        """Test complete concrete table inheritance hierarchy."""

        @inheritance(type=InheritanceType.CONCRETE)
        class Document(Table, ConcreteTableInheritance):
            title: str
            content: str
            created_at: str

            class Settings:
                table_name = "documents"

        class Article(Document):
            title: str
            content: str
            created_at: str
            author: str
            published_date: str

            class Settings:
                table_name = "articles"

        class Report(Document):
            title: str
            content: str
            created_at: str
            department: str
            fiscal_year: int

            class Settings:
                table_name = "reports"

        # Verify inheritance type
        expect(get_inheritance_type(Document)).to_equal(InheritanceType.CONCRETE)

        # Verify independent tables
        expect(Document._table_name).to_equal("documents")
        expect(Article._table_name).to_equal("articles")
        expect(Report._table_name).to_equal("reports")

        # Verify subclass registration
        for base in Article.__mro__[1:]:
            if hasattr(base, "_concrete_subclasses") and isinstance(base, type):
                if issubclass(base, ConcreteTableInheritance):
                    expect(Article in base._concrete_subclasses).to_be_true()
                    expect(Report in base._concrete_subclasses).to_be_true()
                    break

    def test_mixed_mixin_usage(self):
        """Test using multiple mixins together."""
        # Clear registry
        _polymorphic_registry.clear()

        @inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="entity_type")
        class Entity(Table, SingleTableInheritance, PolymorphicQueryMixin):
            name: str
            entity_type: str

            class Settings:
                table_name = "entities"

        class Organization(Entity):
            __discriminator_value__ = "organization"
            tax_id: str

        class Person(Entity):
            __discriminator_value__ = "person"
            ssn: str

        # Verify polymorphic identity
        expect(Organization.polymorphic_identity()).to_equal("organization")
        expect(Person.polymorphic_identity()).to_equal("person")

        # Verify discriminator filter
        org_filter = Organization._get_discriminator_filter()
        expect(org_filter[0]).to_equal("entity_type")
        expect(org_filter[1]).to_equal("organization")

        # Verify get_subclasses
        subclasses = Entity.get_subclasses()
        expect(len(subclasses)).to_equal(2)
        expect(Organization in subclasses).to_be_true()
        expect(Person in subclasses).to_be_true()
