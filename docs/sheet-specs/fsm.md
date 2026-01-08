# Rusheet Finite State Machines (FSM)

This document describes the state models used to manage cell lifecycles and application history.

## Cell Lifecycle FSM

A cell in Rusheet is more than just a value; it has a dynamic state based on its content and validity.

```mermaid
stateDiagram-v2
    [*] --> Empty
    
    Empty --> Literal: User enters text/number
    Empty --> Formula: User enters "="
    
    Literal --> Empty: Delete
    Literal --> Literal: Update value
    Literal --> Formula: Add "=" prefix
    
    state Formula {
        [*] --> Parsing
        Parsing --> Error: Invalid Syntax
        Parsing --> Evaluated: Valid Syntax
        
        Evaluated --> Recalculating: Dependency Changed
        Recalculating --> Evaluated: Success
        Recalculating --> Error: Runtime Error (e.g. Div/0)
        
        Error --> Parsing: Edit Formula
    }
    
    Formula --> Empty: Delete
    Formula --> Literal: Remove "=" prefix
```

### States Definition
1.  **Empty**: The initial state. No content, no formatting (default).
2.  **Literal**: Contains static data (String, Number, Boolean). No dependency tracking needed.
3.  **Formula**:
    *   **Parsing**: The string is being tokenized and parsed into an AST.
    *   **Evaluated**: The formula has been successfully calculated. The cell holds the cached result.
    *   **Recalculating**: A transient state when a dependency has changed, and the cell is waiting for a new value.
    *   **Error**: Represents either a Syntax Error (during parsing) or a Runtime Error (during evaluation, e.g., `#DIV/0!`, `#REF!`).

## History Stack FSM

The Undo/Redo system can be modeled as a linear state machine where the "Current State" pointer moves.

```mermaid
stateDiagram-v2
    state "Undo Stack" as U
    state "Redo Stack" as R
    
    [*] --> Clean
    
    Clean --> Dirty: Command Executed
    Dirty --> Dirty: More Commands
    
    Dirty --> Undo: User Undoes
    Undo --> RedoAvailable: Pointer Moves Back
    
    RedoAvailable --> Dirty: User Redoes (Pointer Moves Forward)
    RedoAvailable --> Dirty: New Command (Clears Redo Stack)
```

### Transitions
*   **Execute Command**: Pushes a new command onto the Undo Stack. **Clears** the Redo Stack.
*   **Undo**: Pops from Undo Stack, executes the inverse operation, pushes to Redo Stack.
*   **Redo**: Pops from Redo Stack, executes the original operation, pushes back to Undo Stack.
