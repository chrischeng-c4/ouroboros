# Rusheet Flowcharts

This document details the critical data flows within Rusheet, specifically focusing on how user updates propagate through the system.

## Cell Update Flow

When a user edits a cell (e.g., typing "42" or "=SUM(A1:B1)"), the following process occurs:

```mermaid
sequenceDiagram
    participant User
    participant UI as React UI
    participant Bridge as WasmBridge
    participant WASM as rusheet-wasm
    participant Hist as rusheet-history
    participant Core as rusheet-core
    participant Form as rusheet-formula

    User->>UI: Types value/formula & hits Enter
    UI->>Bridge: setCellValue(row, col, value)
    Bridge->>WASM: setCellValue(row, col, value)
    
    Note over WASM: Create Command
    WASM->>Hist: push_and_execute(SetCellValueCommand)
    
    activate Hist
    Hist->>Core: update_cell(row, col)
    Core-->>Hist: Success
    deactivate Hist

    Note over WASM: Formula Processing & Recalc
    
    alt is Formula
        WASM->>Form: extract_references(expression)
        WASM->>WASM: Update DependencyGraph
        WASM->>Form: evaluate_formula(expression, get_cell_fn)
        Form-->>WASM: Result
        WASM->>Core: update_cell_cache(result)
    end
    
    WASM->>Form: get_recalc_order(row, col)
    Form-->>WASM: [List of Dependent Cells]
    
    loop For each dependent cell
        WASM->>Core: get_cell(r, c)
        Core-->>WASM: Cell Formula
        WASM->>Form: evaluate_formula(...)
        Form-->>WASM: New Result
        WASM->>Core: update_cell_cache(new_result)
    end
    
    WASM-->>Bridge: JSON [ {r, c}, ... ]
    WASM-->>Bridge: JSON [ {r, c}, ... ]
    Bridge-->>UI: Update Grid
    UI->>User: Renders new values
```

## System Initialization Flow

How the application bootstraps the WASM core:

```mermaid
graph TD
    Start((Start)) --> LoadJS[Load WasmBridge.ts]
    LoadJS --> InitWasm{Wasm Initialized?}
    
    InitWasm -- No --> Fetch[Fetch rusheet_wasm.wasm]
    Fetch --> Compile[Compile/Instantiate WASM]
    Compile --> Bind[Bind JS Imports/Exports]
    Bind --> Create[Create SpreadsheetEngine]
    Create --> Ready((Ready))
    
    InitWasm -- Yes --> Ready
```

## Formula Evaluation Flow

Detailing how a formula string becomes a value:

```mermaid
graph LR
    Input[Input String] --> Check{Starts with =?}
    Check -- No --> Literal[Treat as String/Number]
    
    Check -- Yes --> Lexer
    Lexer --> Tokens
    Tokens --> Parser
    Parser --> AST[Abstract Syntax Tree]
    
    AST --> Evaluator
    Data[Workbook Data] -.-> Evaluator
    
    Evaluator --> Result[Calculated Value]
```
