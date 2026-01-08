---
title: Python Bridge (PyO3)
status: planning
component: data-bridge
type: index
---

# Python Bridge Layer

> **Status**: Documentation Pending

This section will cover the `data-bridge` crate, which handles the FFI (Foreign Function Interface) between Python and Rust using PyO3.

## Key Areas to Document
- Type Conversion (`extracted_to_bson`)
- GIL Management (`Python::allow_threads`)
- Error Mapping (`PyResult`, `PyErr`)
- Module Initialization (`#[pymodule]`)
