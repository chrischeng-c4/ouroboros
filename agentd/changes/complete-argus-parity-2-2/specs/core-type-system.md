# Specification: Core Type System

<meta>
  <constraint>NO actual implementation code - use abstractions only</constraint>
  <abstractions>Mermaid, JSON Schema, Pseudo code, WHEN/THEN</abstractions>
</meta>

## Overview

This specification defines the requirements for achieving 100% feature parity with mypy and pyright for Python type checking. It covers all modern PEPs related to type hinting, bidirectional type inference, and exhaustive type narrowing.

## Requirements

### R1: PEP Parity
The system SHALL support all type hinting features defined in PEP 484, 526, 544, 585, 586, 589, 591, 593, 604, 612, 613, 646, 647, 655, 673, and 742.

### R2: Bidirectional Type Inference
The system SHALL infer types using both context-down (target-typed) and bottom-up (expression-typed) analysis to handle complex generic instantiations and lambda expressions.

### R3: Type Narrowing
The system SHALL support type narrowing via `isinstance`, `issubclass`, `type()`, `match` statements, and user-defined `TypeGuard` / `TypeIs` functions.

### R4: Generic Resolution
The system SHALL correctly resolve generic parameters for classes, functions, and TypeAliases, including support for `ParamSpec` and `TypeVarTuple`.

### R5: Exhaustiveness Checking
The system SHALL verify exhaustive handling of `Union` types and `Enum` members in match statements and conditional blocks when using `assert_never`.

## Data Model

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["kind"],
  "properties": {
    "kind": { "type": "string", "enum": ["Unknown", "Any", "None", "Never", "Instance", "Union", "Callable", "TypeVar", "Protocol"] },
    "name": { "type": "string" },
    "args": { "type": "array", "items": { "$ref": "#" } },
    "origin": { "type": "string", "description": "Fully qualified name of the origin type" },
    "metadata": { "type": "object" }
  }
}
```

## Interfaces

```
FUNCTION check_expression(expr: Expression, context: Context) -> Type
  INPUT: AST expression and current symbol/type context
  OUTPUT: Inferred type of the expression
  ERRORS: TypeError (when types are incompatible)

FUNCTION is_assignable(source: Type, target: Type) -> bool
  INPUT: Source type and target (expected) type
  OUTPUT: True if source can be assigned to target according to subtyping rules
  SIDE_EFFECTS: May trigger generic parameter inference
```

## Acceptance Criteria

### Scenario: Generic List Instantiation
- **WHEN** analyzing `x: list[int] = []`
- **THEN** `x` should be inferred as `list[int]` and the empty list literal should be compatible.

### Scenario: TypeGuard Narrowing
- **WHEN** a variable `val: str | None` is checked with a `TypeGuard[str]` function in an `if` block
- **THEN** the type of `val` inside the `if` block should be `str`.

### Scenario: Exhaustive Match
- **WHEN** matching a `Union[int, str]` without handling the `str` case and calling `assert_never` in the default case
- **THEN** the system should report a type error for non-exhaustive match.
