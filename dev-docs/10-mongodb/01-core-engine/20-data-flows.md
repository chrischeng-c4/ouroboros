---
title: Core MongoDB Data Flows
status: implemented
component: data-bridge-mongodb
type: data-flow
---

# Core MongoDB Data Flows

> Part of [Core MongoDB Engine Documentation](./index.md)

This document illustrates the data flow for key operations, highlighting the interaction between Python, PyO3, and Rust, specifically focusing on GIL management.

## 1. Insert Operation (Write Path)

Goal: Save a Python object to MongoDB efficiently.

```mermaid
sequenceDiagram
    participant Py as Python Runtime
    participant Bind as PyO3 Binding
    participant Rust as Rust Core
    participant DB as MongoDB

    Py->>Bind: doc.save()
    Note right of Py: GIL HELD

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 1: Extraction
        Bind->>Bind: extract_py_value(self)<br/>(PyObject -> ExtractedValue)
        Note right of Bind: Fast, memory-only
    end

    Bind->>Rust: async insert(extracted)
    Note right of Py: GIL RELEASED

    rect rgb(200, 255, 200)
        Note right of Rust: Phase 2: Conversion & I/O
        Rust->>Rust: extracted_to_bson()<br/>(ExtractedValue -> Bson)
        Rust->>Rust: validate_document()
        Rust->>DB: insert_one(bson)
        DB-->>Rust: result (ObjectId)
    end

    Rust-->>Bind: result
    Note right of Py: GIL ACQUIRED

    Bind-->>Py: return result
```

## 2. Find Operation (Read Path)

Goal: Retrieve documents and convert them to Python objects.

```mermaid
sequenceDiagram
    participant Py as Python Runtime
    participant Bind as PyO3 Binding
    participant Rust as Rust Core
    participant DB as MongoDB

    Py->>Bind: find_one(query)
    Note right of Py: GIL HELD

    Bind->>Rust: async find(query)
    Note right of Py: GIL RELEASED

    rect rgb(200, 255, 200)
        Note right of Rust: Phase 1: I/O & BSON
        Rust->>DB: find_one(bson_query)
        DB-->>Rust: BsonDocument
    end

    Rust-->>Bind: BsonDocument
    Note right of Py: GIL ACQUIRED

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 2: Object Creation
        Bind->>Bind: bson_to_py(bson)<br/>(Bson -> PyObject)
        Note right of Bind: Construct dicts/lists
    end

    Bind-->>Py: return Python Object
```

## 3. Bulk Insert (Parallel Optimization)

Goal: Insert 10,000 documents as fast as possible.

```mermaid
sequenceDiagram
    participant Py as Python Runtime
    participant Bind as PyO3 Binding
    participant Rayon as Rust Rayon Pool
    participant DB as MongoDB

    Py->>Bind: insert_many(docs)
    Note right of Py: GIL HELD

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 1: Serial Extraction
        Bind->>Bind: Loop docs: extract_py_value()
        Note right of Bind: Creates Vec<ExtractedValue>
    end

    Bind->>Rayon: spawn_parallel(extracted_list)
    Note right of Py: GIL RELEASED

    rect rgb(200, 255, 200)
        Note right of Rayon: Phase 2: Parallel Conversion
        Rayon->>Rayon: par_iter().map(extracted_to_bson)
        Note right of Rayon: Uses all CPU cores
        Rayon->>DB: insert_many(bson_list)
        DB-->>Rayon: results
    end

    Rayon-->>Bind: results
    Note right of Py: GIL ACQUIRED

    Bind-->>Py: return results
```

## Data Transformation Pipeline

The following diagram shows how data types are transformed at each stage.

```
[ Python Layer ]       [ PyO3 Bridge ]                  [ Rust Core ]              [ MongoDB ]
  dict {                 ExtractedValue::Dict {           Bson::Document {           BSON Bytes
    "name": "A",   ->      "name": String("A"),     ->      "name": String("A"),  ->   \x16\x00...
    "age": 30              "age": Int(30)                   "age": Int32(30)
  }                      }                                }
      |                          |                                |
      | (GIL Held)               | (GIL Released)                 | (Network)
      +------------------------> +------------------------------> +
         Extraction                  Conversion                     Serialization
```

## Error Propagation Flow

```mermaid
sequenceDiagram
    participant Rust
    participant PyO3
    participant Python

    Rust->>Rust: MongoError (Connection Failed)
    Rust->>PyO3: Result::Err(Error::Mongo(...))
    PyO3->>PyO3: Map Error -> PyErr (ConnectionError)
    PyO3->>Python: Raise Exception
    Note over Python: try/except block catches it
```
