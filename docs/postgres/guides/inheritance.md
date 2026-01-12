# PostgreSQL Table Inheritance Patterns

The `data_bridge.postgres.inheritance` module provides SQLAlchemy-style table inheritance patterns for the PostgreSQL ORM. This enables you to model class hierarchies in your database using three different strategies.

## Overview

### Inheritance Strategies

1. **Single Table Inheritance (SINGLE_TABLE)**
   - All classes in hierarchy share one table
   - Uses discriminator column to distinguish types
   - Fastest queries (no JOINs)
   - May have NULL columns for unused fields

2. **Joined Table Inheritance (JOINED)**
   - Each class has its own table
   - Child tables have FK to parent table
   - Normalized schema (no NULL waste)
   - Requires JOINs for queries

3. **Concrete Table Inheritance (CONCRETE)**
   - Each class has complete standalone table
   - No foreign keys between tables
   - Fastest for single-class queries
   - Requires UNIONs for polymorphic queries

## Installation

The inheritance module is part of the postgres package:

```python
from data_bridge.postgres import (
    InheritanceType,
    inheritance,
    SingleTableInheritance,
    JoinedTableInheritance,
    ConcreteTableInheritance,
    PolymorphicQueryMixin,
)
```

## Basic Usage

### Single Table Inheritance

All subclasses share one physical table with a discriminator column:

```python
from data_bridge.postgres import Table, Column, InheritanceType, inheritance, SingleTableInheritance

@inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="employee_type")
class Employee(Table, SingleTableInheritance):
    name: str
    email: str
    employee_type: str  # Discriminator column

    class Settings:
        table_name = "employees"

class Manager(Employee):
    __discriminator_value__ = "manager"
    department: str
    budget: int

class Engineer(Employee):
    __discriminator_value__ = "engineer"
    programming_language: str
    seniority_level: str
```

**Database Schema:**

```sql
CREATE TABLE employees (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL,
    employee_type VARCHAR NOT NULL,  -- Discriminator
    department VARCHAR,               -- NULL for non-managers
    budget INTEGER,                   -- NULL for non-managers
    programming_language VARCHAR,     -- NULL for non-engineers
    seniority_level VARCHAR          -- NULL for non-engineers
);
```

**Usage:**

```python
# Create instances
manager = Manager(
    name="Alice",
    email="alice@example.com",
    employee_type="manager",
    department="Engineering",
    budget=500000
)
await manager.save()

# Query - automatically filters by discriminator
managers = await Manager.find().to_list()
# SELECT * FROM employees WHERE employee_type = 'manager'

engineers = await Engineer.find().to_list()
# SELECT * FROM employees WHERE employee_type = 'engineer'
```

**Pros:**
- Fast queries (no JOINs)
- Simple schema
- Easy to add new subclasses

**Cons:**
- NULL columns for unused fields
- All columns must be nullable (except discriminator)
- Larger table size

### Joined Table Inheritance

Each class has its own table, joined via foreign key:

```python
from data_bridge.postgres import Table, Column, InheritanceType, inheritance, JoinedTableInheritance

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
```

**Database Schema:**

```sql
CREATE TABLE vehicles (
    id SERIAL PRIMARY KEY,
    make VARCHAR NOT NULL,
    model VARCHAR NOT NULL,
    year INTEGER NOT NULL
);

CREATE TABLE cars (
    id INTEGER PRIMARY KEY REFERENCES vehicles(id) ON DELETE CASCADE,
    num_doors INTEGER NOT NULL,
    trunk_size FLOAT NOT NULL
);

CREATE TABLE trucks (
    id INTEGER PRIMARY KEY REFERENCES vehicles(id) ON DELETE CASCADE,
    bed_length FLOAT NOT NULL,
    towing_capacity INTEGER NOT NULL
);
```

**Usage:**

```python
# Create instances
car = Car(
    make="Toyota",
    model="Camry",
    year=2023,
    num_doors=4,
    trunk_size=15.1
)
await car.save()
# Inserts into both 'vehicles' and 'cars' tables

# Query - automatically JOINs parent table
cars = await Car.find().to_list()
# SELECT c.*, v.* FROM cars c
# JOIN vehicles v ON c.id = v.id
```

**Pros:**
- Normalized schema (no NULL waste)
- Clear separation of concerns
- Type-specific columns

**Cons:**
- Slower queries (requires JOINs)
- More complex schema
- INSERT/UPDATE affects multiple tables

### Concrete Table Inheritance

Each class has a complete standalone table:

```python
from data_bridge.postgres import Table, Column, InheritanceType, inheritance, ConcreteTableInheritance

@inheritance(type=InheritanceType.CONCRETE)
class Animal(Table, ConcreteTableInheritance):
    name: str
    age: int

    class Settings:
        table_name = "animals"

class Dog(Animal):
    name: str  # Duplicated from parent
    age: int   # Duplicated from parent
    breed: str
    is_good_boy: bool

    class Settings:
        table_name = "dogs"

class Cat(Animal):
    name: str  # Duplicated from parent
    age: int   # Duplicated from parent
    fur_color: str
    indoor_only: bool

    class Settings:
        table_name = "cats"
```

**Database Schema:**

```sql
-- Each table is completely independent
CREATE TABLE dogs (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    age INTEGER NOT NULL,
    breed VARCHAR NOT NULL,
    is_good_boy BOOLEAN DEFAULT TRUE
);

CREATE TABLE cats (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    age INTEGER NOT NULL,
    fur_color VARCHAR NOT NULL,
    indoor_only BOOLEAN NOT NULL
);
```

**Usage:**

```python
# Create instances
dog = Dog(
    name="Buddy",
    age=5,
    breed="Golden Retriever",
    is_good_boy=True
)
await dog.save()
# Only inserts into 'dogs' table

# Query - only queries specific table
dogs = await Dog.find().to_list()
# SELECT * FROM dogs

# Polymorphic query - uses UNION
all_animals = await Animal.find_polymorphic().to_list()
# SELECT *, 'dog' as _type FROM dogs
# UNION ALL
# SELECT *, 'cat' as _type FROM cats
```

**Pros:**
- Fast single-class queries (no JOINs)
- Complete independence
- Can have different schemas per subclass

**Cons:**
- Duplicated columns
- Schema changes must be applied to all tables
- UNION queries for polymorphic loading

## API Reference

### InheritanceType (Enum)

```python
class InheritanceType(Enum):
    SINGLE_TABLE = "single_table"
    JOINED = "joined"
    CONCRETE = "concrete"
```

### InheritanceConfig (Dataclass)

Configuration for table inheritance:

```python
@dataclass
class InheritanceConfig:
    inheritance_type: InheritanceType
    discriminator_column: str = "type"
    discriminator_value: Optional[str] = None
    polymorphic_on: Optional[str] = None
```

### @inheritance Decorator

Configures inheritance strategy for a base class:

```python
@inheritance(
    type: InheritanceType = InheritanceType.SINGLE_TABLE,
    discriminator: str = "type",
    polymorphic_on: Optional[str] = None
)
```

**Parameters:**
- `type`: Inheritance strategy to use
- `discriminator`: Name of discriminator column (SINGLE_TABLE only)
- `polymorphic_on`: Column to distinguish types (defaults to discriminator)

### Mixin Classes

#### SingleTableInheritance

Base mixin for single table inheritance:

```python
class Employee(Table, SingleTableInheritance):
    # Automatically adds discriminator filtering to queries
    pass
```

**Class Attribute:**
- `__discriminator_value__`: Value in discriminator column for this class

**Class Methods:**
- `_get_discriminator_filter()`: Returns (column, value) tuple for filtering
- `polymorphic_identity()`: Returns discriminator value

#### JoinedTableInheritance

Base mixin for joined table inheritance:

```python
class Vehicle(Table, JoinedTableInheritance):
    # Automatically adds JOINs to parent table in queries
    pass
```

**Class Methods:**
- `_get_join_config()`: Returns JOIN configuration dict

#### ConcreteTableInheritance

Base mixin for concrete table inheritance:

```python
class Animal(Table, ConcreteTableInheritance):
    # Each subclass has independent table
    pass
```

**Class Attribute:**
- `_concrete_subclasses`: List of registered subclasses

#### PolymorphicQueryMixin

Adds polymorphic query methods:

```python
class Employee(Table, SingleTableInheritance, PolymorphicQueryMixin):
    pass
```

**Class Methods:**
- `fetch_polymorphic(*conditions, limit)`: Fetch objects as correct subclass
- `polymorphic_identity()`: Returns discriminator value
- `get_subclasses()`: Returns all registered subclasses

### Helper Functions

#### get_inheritance_type(cls)

Get the inheritance type for a class:

```python
inh_type = get_inheritance_type(Employee)
# Returns: InheritanceType.SINGLE_TABLE
```

#### get_discriminator_column(cls)

Get the discriminator column name:

```python
col_name = get_discriminator_column(Employee)
# Returns: "employee_type"
```

#### get_discriminator_value(cls)

Get the discriminator value for a class:

```python
value = get_discriminator_value(Manager)
# Returns: "manager"
```

#### register_polymorphic_class(parent, child, discriminator_value)

Manually register a polymorphic subclass:

```python
register_polymorphic_class(Employee, Manager, "manager")
```

#### get_polymorphic_map(cls)

Get mapping of discriminator values to classes:

```python
poly_map = get_polymorphic_map(Employee)
# Returns: {"manager": Manager, "engineer": Engineer, ...}

# Use to instantiate correct subclass
employee_type = row["type"]
cls = poly_map.get(employee_type, Employee)
employee = cls(**row)
```

## Advanced Usage

### Polymorphic Queries

Query all instances and get them as their correct subclass:

```python
# Single table inheritance
employees = await Employee.fetch_polymorphic()
for emp in employees:
    if isinstance(emp, Manager):
        print(f"Manager: {emp.name}, Dept: {emp.department}")
    elif isinstance(emp, Engineer):
        print(f"Engineer: {emp.name}, Lang: {emp.programming_language}")
```

### Dynamic Type Resolution

Use the polymorphic map to dynamically instantiate the correct class:

```python
poly_map = get_polymorphic_map(Employee)

# From database row
row = {"employee_type": "manager", "name": "Alice", "department": "Engineering"}
employee_class = poly_map.get(row["employee_type"], Employee)
employee = employee_class(**row)

print(type(employee))  # <class 'Manager'>
```

### Multiple Inheritance Levels

You can have multiple levels of inheritance:

```python
@inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
class Person(Table, SingleTableInheritance):
    name: str
    type: str

class Employee(Person):
    __discriminator_value__ = "employee"
    company: str

class Manager(Employee):
    __discriminator_value__ = "manager"
    department: str

class Engineer(Employee):
    __discriminator_value__ = "engineer"
    language: str
```

## Best Practices

### When to Use Each Strategy

**Use Single Table Inheritance when:**
- Few subclasses with similar fields
- Query performance is critical
- Schema is relatively stable
- Don't mind some NULL columns

**Use Joined Table Inheritance when:**
- Many subclasses with different fields
- Database normalization is important
- Fields vary significantly between types
- Okay with JOIN overhead

**Use Concrete Table Inheritance when:**
- Subclasses are completely independent
- No polymorphic queries needed
- Maximum query performance per type
- Schema can differ significantly

### Discriminator Column Tips

1. **Make it NOT NULL**: Always require the discriminator
2. **Add an index**: Speeds up type-filtered queries
3. **Use VARCHAR**: Allows readable values like "manager"
4. **Consider ENUM**: PostgreSQL ENUMs provide type safety

```sql
CREATE TYPE employee_type_enum AS ENUM ('manager', 'engineer', 'contractor');

CREATE TABLE employees (
    id SERIAL PRIMARY KEY,
    employee_type employee_type_enum NOT NULL,
    -- other columns
);

CREATE INDEX idx_employee_type ON employees(employee_type);
```

### Validation

Always validate discriminator values match the class:

```python
class Manager(Employee):
    __discriminator_value__ = "manager"

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        if self.employee_type != "manager":
            raise ValueError("employee_type must be 'manager' for Manager instances")
```

## Migration Guide

### From SQLAlchemy

If migrating from SQLAlchemy, the patterns are very similar:

**SQLAlchemy:**
```python
class Employee(Base):
    __tablename__ = 'employees'
    id = Column(Integer, primary_key=True)
    type = Column(String(50))

    __mapper_args__ = {
        'polymorphic_identity': 'employee',
        'polymorphic_on': type
    }

class Manager(Employee):
    __mapper_args__ = {
        'polymorphic_identity': 'manager',
    }
```

**data-bridge:**
```python
@inheritance(type=InheritanceType.SINGLE_TABLE, discriminator="type")
class Employee(Table, SingleTableInheritance):
    type: str

    class Settings:
        table_name = "employees"

class Manager(Employee):
    __discriminator_value__ = "manager"
```

## Performance Considerations

### Single Table Inheritance

- **Fastest** for queries that filter by type
- **Index** the discriminator column
- **Monitor** table size (can get large with many types)

### Joined Table Inheritance

- **JOIN** overhead on every query
- **Consider** eager loading to avoid N+1 queries
- **Index** foreign key columns

### Concrete Table Inheritance

- **Fastest** for single-type queries
- **Slowest** for polymorphic queries (UNION)
- **Consider** table partitioning for large datasets

## Limitations

1. **No runtime type switching**: Once an instance is created with a discriminator, it cannot change type
2. **Schema migrations**: Changing inheritance strategy requires data migration
3. **Query builder integration**: Some advanced query features may need updates to support inheritance
4. **Polymorphic queries**: `fetch_polymorphic()` requires integration with query builder (placeholder in current implementation)

## Future Enhancements

Planned improvements:

1. **Query builder integration**: Automatic discriminator filtering
2. **Polymorphic eager loading**: Load relationships polymorphically
3. **Migration helpers**: Tools to convert between strategies
4. **Type coercion**: Automatic casting to correct subclass
5. **Validation hooks**: Built-in discriminator validation

## Examples

See `/Users/chrischeng/projects/data-bridge/examples/inheritance_example.py` for complete working examples.

## Related Documentation

- [PostgreSQL Table Documentation](./postgres_table.md)
- [Query Builder Documentation](./postgres_query.md)
- [Relationships Documentation](./postgres_relationships.md)
