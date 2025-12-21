# Python Layer Data Flows

> Part of [Python API Layer Documentation](./README.md)

## 1. Query Construction Flow

Goal: Transform `User.find(User.age > 18).sort("-name")` into a MongoDB query.

```mermaid
sequenceDiagram
    participant User as User Code
    participant Proxy as FieldProxy
    participant Builder as QueryBuilder
    participant Rust as Rust Engine

    User->>Proxy: User.age > 18
    Proxy-->>User: QueryExpr({"age": {"$gt": 18}})

    User->>Builder: User.find(expr)
    Builder->>Builder: store filter

    User->>Builder: .sort("-name")
    Builder->>Builder: parse sort string<br/>[("name", -1)]

    User->>Builder: .to_list()
    Builder->>Rust: find(filter, sort, ...)
    Rust-->>Builder: List[RustDocument]
    Builder-->>User: List[User]
```

## 2. Document Hydration Flow

Goal: Convert raw data from Rust into a fully functional `User` instance.

```mermaid
sequenceDiagram
    participant Rust as Rust Engine
    participant Meta as DocumentMeta
    participant Doc as Document Instance
    participant State as StateTracker

    Rust->>Meta: _from_db(data: dict)
    Meta->>Doc: __new__(cls)
    Note over Doc: Create bare instance

    Meta->>Doc: __dict__.update(data)
    Note over Doc: Bulk populate fields<br/>(Bypass __setattr__)

    Meta->>State: initialize(data)
    Note over State: Store baseline state<br/>(Copy-on-Write)

    Meta-->>Rust: return instance
```

## 3. Save Lifecycle (Update)

Goal: Save a modified document.

```mermaid
sequenceDiagram
    participant User as User Code
    participant Doc as Document
    participant State as StateTracker
    participant Engine as _engine.py
    participant Rust as Rust Backend

    User->>Doc: user.age = 31
    Doc->>State: mark_changed("age")

    User->>Doc: user.save()
    
    Doc->>State: get_changes()
    State-->>Doc: {"age": 31}

    alt No Changes
        Doc-->>User: return (no-op)
    else Has Changes
        Doc->>Doc: validate() (Optional)
        Doc->>Engine: update_one(id, {"$set": {"age": 31}})
        Engine->>Rust: update_one(...)
        Rust-->>Engine: Result
        
        Doc->>State: commit()
        Note over State: Update baseline
        
        Doc-->>User: return Result
    end
```
