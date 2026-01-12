# Validation

The ORM provides a validation system similar to SQLAlchemy, allowing you to ensure data integrity before it reaches the database.

## Field Validation

Use the `@validates` decorator to validate specific fields.

```python
from data_bridge.postgres import Table, validates, ValidationError

class User(Table):
    email: str
    username: str

    @validates("email")
    def validate_email(self, key, value):
        if "@" not in value:
            raise ValidationError(key, "Invalid email format")
        
        # Validators can also normalize data
        return value.lower()
```

The validation method receives:
- `key`: The name of the field being validated.
- `value`: The value being assigned.

It must return the value (or a modified version of it). If validation fails, raise `ValidationError` (or `ValueError`).

## Multi-Field Validation

Use `@validates_many` to validate dependencies between fields.

```python
from data_bridge.postgres import validates_many

class ChangePasswordRequest(Table):
    password: str
    confirm_password: str

    @validates_many("password", "confirm_password")
    def validate_match(self, values):
        if values.get("password") != values.get("confirm_password"):
            raise ValidationError("confirm_password", "Passwords do not match")
        return values
```

## Built-in Validators

`data-bridge` comes with a set of common validators to save you time.

```python
from data_bridge.postgres import Table, validates
from data_bridge.postgres.validation import validate_email, validate_range

class Profile(Table):
    email: str
    age: int

    @validates("email")
    def email_check(self, key, value):
        if not validate_email(value):
            raise ValueError("Bad email")
        return value

    @validates("age")
    def age_check(self, key, value):
        validate_range(value, 18, 120) # Raises ValueError if invalid
        return value
```

### Available Validators

- `validate_not_empty(value)`
- `validate_email(value)`
- `validate_url(value)`
- `validate_min_length(value, min)`
- `validate_max_length(value, max)`
- `validate_regex(value, pattern)`
- `validate_range(value, min, max)`
- `validate_min_value(value, min)`
- `validate_max_value(value, max)`
- `validate_in_list(value, choices)`
