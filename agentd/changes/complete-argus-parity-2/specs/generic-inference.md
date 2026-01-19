# Specification: Generic Class Instantiation Inference

<meta>
  <constraint>NO actual implementation code - use abstractions only</constraint>
  <abstractions>Mermaid, JSON Schema, Pseudo code, WHEN/THEN</abstractions>
</meta>

## Overview

Improve type inference to correctly handle generic class instantiations. When a generic class is instantiated, the type parameters should be inferred from the constructor arguments.

## Requirements

### R1: Constructor-based Inference
When instantiating a generic class (e.g., `Stack([1, 2, 3])`), Argus should infer the type parameters (e.g., `T=int`) based on the types of the arguments passed to `__init__`.

### R2: Nested Generics
Support inference for nested generic types (e.g., `Box(List([1]))` -> `Box[List[int]]`).

### R3: TypeVar Constraints and Bounds
Honor constraints (e.g., `TypeVar('T', int, str)`) and bounds (e.g., `TypeVar('T', bound=Buffer)`) during inference.

### R4: Class Metadata Extensions
Extend the internal `ClassInfo` structure to explicitly track generic parameters, their declaration order, and variance. This is required to map constructor arguments back to the correct type parameters.

## Acceptance Criteria

### Scenario: WHEN instantiate generic THEN infer type parameter
- **WHEN** user writes `x = List([1, 2, 3])` where `List` is defined as `class List(Generic[T])`
- **THEN** Argus infers the type of `x` as `List[int]`

### Scenario: WHEN cannot infer THEN default to Any
- **WHEN** type parameters cannot be inferred from arguments and no defaults are provided
- **THEN** Argus defaults the type arguments to `Any`

### Scenario: WHEN multiple type params THEN infer all
- **WHEN** user writes `d = Dict("key", 42)` where `Dict` is `Generic[K, V]`
- **THEN** Argus infers `Dict[str, int]`

### Scenario: WHEN violate constraints THEN error
- **WHEN** `T` is constrained to `int | str` and user writes `Wrapper(1.5)`
- **THEN** Argus reports a type error because `float` does not satisfy the constraints
