# Specification: Parser Error Recovery

<meta>
  <constraint>NO actual implementation code - use abstractions only</constraint>
  <abstractions>Mermaid, JSON Schema, Pseudo code, WHEN/THEN</abstractions>
</meta>

## Overview

In an interactive development environment, code is frequently in an invalid state (e.g., while the user is typing). This specification defines how the Argus parser should recover from syntax errors to provide continuous semantic analysis for the valid parts of the file.

## Requirements

### R1: Statement-level Recovery
If a syntax error occurs within a statement, the parser SHALL attempt to skip to the next valid statement (e.g., by looking for newlines or indentation changes) and resume parsing.

### R2: Block-level Synchronization
For nested structures (classes, functions, if-blocks), the parser SHALL use indentation and keywords (`def`, `class`, `pass`) as synchronization points to recover from malformed blocks.

### R3: Error Tolerant AST
The AST (Abstract Syntax Tree) SHALL be able to represent "Error" nodes so that subsequent semantic analysis passes can safely ignore invalid branches while still processing the rest of the tree.

### R4: Partial Semantic Analysis
Semantic analysis (symbol collection, type checking) SHALL continue even if parts of the AST contain error nodes, providing diagnostics for valid code sections.

## Acceptance Criteria

### Scenario: Typing inside a Function
- **WHEN** a user is in the middle of typing a statement inside `def my_func():`
- **THEN** other functions in the same file should still have their symbols collected and type checked correctly.

### Scenario: Unclosed Parentheses
- **WHEN** a function call has an unclosed parenthesis `print("hello"`
- **THEN** the parser should report the syntax error but still allow the rest of the module to be analyzed.

### Scenario: Indentation Error
- **WHEN** a single line has an indentation error
- **THEN** Argus should report the error but attempt to sync with the next line that matches the previous valid indentation level.
