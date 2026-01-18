# Specification: Variance Checking

<meta>
  <constraint>NO actual implementation code - use abstractions only</constraint>
  <abstractions>Mermaid, JSON Schema, Pseudo code, WHEN/THEN</abstractions>
</meta>

## Overview

Implement variance checking (covariance, contravariance, invariance) for generic types to ensure type safety in complex hierarchies. This is essential for correct handling of generic containers and callable types.

## Requirements

### R1: TypeVar Variance Declaration
Support `covariant=True` and `contravariant=True` in `TypeVar` declarations.

### R2: Subtyping Rules
Implement subtyping logic for generic types:
- `List[T]` is invariant (default).
- `Sequence[T]` is covariant.
- `Callable[[T], None]` is contravariant in `T`.

### R3: Assignment Validation
Validate assignments between generic types based on their variance.

## Data Model

```json
{
  "type": "string",
  "enum": ["Invariant", "Covariant", "Contravariant", "Bivariant"]
}
```

## Acceptance Criteria

### Scenario: WHEN covariant assignment THEN accept
- **WHEN** `Employee` is a subclass of `Person`, and a variable of type `Sequence[Person]` is assigned a value of type `Sequence[Employee]`
- **THEN** Argus accepts the assignment (since `Sequence` is covariant)

### Scenario: WHEN invariant mismatch THEN error
- **WHEN** `Employee` is a subclass of `Person`, and `List[Person]` is assigned `List[Employee]`
- **THEN** Argus reports a type error (since `List` is invariant)

### Scenario: WHEN contravariant callable THEN accept
- **WHEN** a function expects `Callable[[Employee], None]` and is given `Callable[[Person], None]`
- **THEN** Argus accepts the assignment (since `Callable` is contravariant in its arguments)

### Scenario: WHEN covariant in contravariant position THEN error
- **WHEN** a covariant `TypeVar` is used as an argument type in a method
- **THEN** Argus reports a "Variance error: covariant type variable used in contravariant position"
