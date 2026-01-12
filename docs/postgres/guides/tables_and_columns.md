# Tables & Columns

## Defining Tables

Tables are defined as Python classes inheriting from `data_bridge.postgres.Table`. Each table maps to a PostgreSQL table, and instances of the class represent rows.

```python
from data_bridge.postgres import Table

class Product(Table):
    name: str
    price: float
```

### Table Settings

You can configure table metadata using the `Settings` inner class:

```python
class Product(Table):
    # ... columns ...

    class Settings:
        table_name = "products"       # Default: class name lowercase
        schema = "store"              # Default: "public"
        primary_key = "id"            # Default: "id"
        indexes = [
            {"columns": ["name"], "unique": True}
        ]
```

## Columns

Columns are defined using Python type hints. The ORM automatically maps these to PostgreSQL types.

### Basic Types

| Python Type | PostgreSQL Type |
|-------------|-----------------|
| `str`       | `VARCHAR` / `TEXT` |
| `int`       | `INTEGER`       |
| `float`     | `DOUBLE PRECISION` |
| `bool`      | `BOOLEAN`       |
| `datetime`  | `TIMESTAMP`     |
| `date`      | `DATE`          |
| `Decimal`   | `NUMERIC`       |
| `dict`      | `JSONB`         |

### Column Options

Use the `Column()` helper to define constraints and default values.

```python
from data_bridge.postgres import Table, Column
from datetime import datetime

class User(Table):
    # Standard column
    username: str = Column(unique=True)

    # Nullable column
    bio: str | None = None

    # Default value (static)
    is_active: bool = True

    # Default factory (dynamic)
    created_at: datetime = Column(default_factory=datetime.now)
```

## Computed Columns

`data-bridge` supports computed properties that can be used in Python logic and (optionally) persisted or calculated by the database (if supported by the backend logic, currently focused on Python-side computation with `Computed`).

```python
from data_bridge.postgres import Table, Computed

class Rectangle(Table):
    width: float
    height: float

    @Computed
    def area(self) -> float:
        return self.width * self.height
```

## Auto-Coercion

The `AutoCoerceMixin` allows automatic conversion of input data to the correct Python types. This is useful when working with API inputs.

```python
from data_bridge.postgres import Table
from data_bridge.postgres.validation import AutoCoerceMixin

class Item(AutoCoerceMixin, Table):
    quantity: int
    price: float

# Inputs are strings, but they get converted
item = Item(quantity="5", price="19.99")

assert item.quantity == 5       # int
assert item.price == 19.99      # float
```
