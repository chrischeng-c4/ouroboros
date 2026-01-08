---
title: Core PostgreSQL Components
status: planning
component: postgres
type: components
---

# Core PostgreSQL Components

> Part of [Core PostgreSQL Engine Documentation](./index.md)

This document details the responsibilities and internals of the key components in the PostgreSQL engine.

## 1. Connection Manager (`connection.rs`)

**Role**: Manages the lifecycle of PostgreSQL connection pools and provides database access.

**Key Responsibilities**:
- **Pooling**: Wraps `sqlx::PgPool` with configurable min/max connections, timeouts, and idle handling.
- **Configuration**: Handles connection strings, SSL/TLS settings, and connection limits.
- **Health Checks**: Provides methods to `ping` the database and check server status.
- **Transaction Support**: Creates transaction contexts for atomic operations.

**Rust Types**:
```rust
pub struct PoolConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
    pub require_ssl: bool,
}

pub struct Connection {
    pool: PgPool,
    config: PoolConfig,
}

impl Connection {
    pub async fn new(uri: &str) -> Result<Self>;
    pub async fn with_config(uri: &str, config: PoolConfig) -> Result<Self>;

    pub fn pool(&self) -> &PgPool;
    pub async fn ping(&self) -> Result<()>;
    pub async fn server_version(&self) -> Result<String>;

    pub async fn begin(&self) -> Result<Transaction>;
    pub async fn execute(&self, query: &str) -> Result<PgQueryResult>;
}
```

**Example**:
```rust
let config = PoolConfig {
    min_connections: 5,
    max_connections: 20,
    connect_timeout: Duration::from_secs(10),
    idle_timeout: Some(Duration::from_secs(300)),
    max_lifetime: Some(Duration::from_secs(1800)),
    require_ssl: true,
};

let conn = Connection::with_config(
    "postgresql://user:pass@localhost/mydb",
    config
).await?;

// Health check
conn.ping().await?;
```

## 2. Row Trait (`row.rs`)

**Role**: The abstraction for any entity that maps to a PostgreSQL table row.

**Key Responsibilities**:
- **CRUD Interface**: Defines `insert`, `find_by_id`, `find_where`, `update`, `delete` operations.
- **Type Safety**: Leverages `sqlx::FromRow` for automatic row-to-struct mapping.
- **Schema Binding**: Each implementation is bound to a specific table name and primary key.
- **Column Mapping**: Provides static metadata about table structure.

**Signature**:
```rust
#[async_trait]
pub trait Row: sqlx::FromRow<'static, PgRow> + Send + Sync + Sized {
    // Static metadata
    fn table_name() -> &'static str;
    fn primary_key_column() -> &'static str {
        "id"  // Default
    }
    fn columns() -> &'static [&'static str];

    // Instance methods
    fn primary_key_value(&self) -> &dyn ToSql;

    // CRUD operations
    async fn insert(&self, conn: &Connection) -> Result<Self>;
    async fn find_by_id<T: ToSql>(conn: &Connection, id: T) -> Result<Option<Self>>;
    async fn find_where(conn: &Connection, condition: &WhereClause) -> Result<Vec<Self>>;
    async fn update(&self, conn: &Connection) -> Result<Self>;
    async fn delete(&self, conn: &Connection) -> Result<u64>;

    // Bulk operations
    async fn insert_many(conn: &Connection, rows: Vec<Self>) -> Result<Vec<Self>>;
    async fn delete_where(conn: &Connection, condition: &WhereClause) -> Result<u64>;
}
```

**Example Implementation**:
```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl Row for User {
    fn table_name() -> &'static str {
        "users"
    }

    fn columns() -> &'static [&'static str] {
        &["id", "email", "name", "created_at"]
    }

    fn primary_key_value(&self) -> &dyn ToSql {
        &self.id
    }
}

// Usage
let user = User::find_by_id(&conn, 42).await?;
user.update(&conn).await?;
```

## 3. Query Builder (`query.rs`)

**Role**: Provides a fluent, type-safe API for constructing SQL queries.

**Key Responsibilities**:
- **Chaining**: Methods like `.where_eq()`, `.order_by()`, `.limit()` return `Self`.
- **Type Safety**: Prevents SQL injection through parameterized queries.
- **Operator Support**: Comprehensive set of comparison and logical operators.
- **SQL Generation**: Builds PostgreSQL-compatible SQL with numbered parameters (`$1`, `$2`, etc.).

**Rust Types**:
```rust
pub enum Operator {
    Eq,      // =
    Ne,      // !=
    Lt,      // <
    Le,      // <=
    Gt,      // >
    Ge,      // >=
    Like,    // LIKE
    ILike,   // ILIKE (case-insensitive)
    In,      // IN
    Between, // BETWEEN
    IsNull,  // IS NULL
    IsNotNull, // IS NOT NULL
}

pub struct WhereClause {
    column: String,
    operator: Operator,
    value: Option<SqlValue>,
}

pub struct QueryBuilder {
    table: String,
    select_columns: Vec<String>,
    where_clauses: Vec<WhereClause>,
    order_by: Vec<(String, SortOrder)>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl QueryBuilder {
    pub fn new(table: &str) -> Self;

    // SELECT clause
    pub fn select(mut self, columns: &[&str]) -> Self;

    // WHERE clause
    pub fn where_eq(mut self, column: &str, value: impl ToSql) -> Self;
    pub fn where_ne(mut self, column: &str, value: impl ToSql) -> Self;
    pub fn where_lt(mut self, column: &str, value: impl ToSql) -> Self;
    pub fn where_like(mut self, column: &str, pattern: &str) -> Self;
    pub fn where_in(mut self, column: &str, values: Vec<impl ToSql>) -> Self;
    pub fn where_null(mut self, column: &str) -> Self;

    // ORDER BY clause
    pub fn order_by(mut self, column: &str, order: SortOrder) -> Self;

    // LIMIT/OFFSET
    pub fn limit(mut self, limit: u64) -> Self;
    pub fn offset(mut self, offset: u64) -> Self;

    // Build query
    pub fn build(&self) -> (String, Vec<SqlValue>);

    // Execute
    pub async fn fetch_all<T: Row>(&self, conn: &Connection) -> Result<Vec<T>>;
    pub async fn fetch_one<T: Row>(&self, conn: &Connection) -> Result<Option<T>>;
}
```

**Example Usage**:
```rust
// Build a complex query
let (sql, params) = QueryBuilder::new("users")
    .select(&["id", "email", "name"])
    .where_eq("status", "active")
    .where_like("email", "%@example.com")
    .order_by("created_at", SortOrder::Desc)
    .limit(10)
    .offset(20)
    .build();

// Generated SQL:
// SELECT id, email, name FROM users
// WHERE status = $1 AND email LIKE $2
// ORDER BY created_at DESC
// LIMIT 10 OFFSET 20

// Direct execution
let users: Vec<User> = QueryBuilder::new("users")
    .where_eq("department", "engineering")
    .order_by("name", SortOrder::Asc)
    .fetch_all(&conn)
    .await?;
```

## 4. Transaction Manager (`transaction.rs`)

**Role**: Manages database transactions with savepoint support.

**Key Responsibilities**:
- **Atomicity**: Ensures all-or-nothing execution of operations.
- **Savepoint Support**: Allows partial rollback within transactions.
- **Nested Transactions**: Supports savepoint-based nesting.
- **Auto-Rollback**: Automatically rolls back on drop if not committed.

**Rust Types**:
```rust
pub struct Transaction<'a> {
    tx: sqlx::Transaction<'a, Postgres>,
    savepoints: Vec<String>,
}

impl<'a> Transaction<'a> {
    pub async fn commit(self) -> Result<()>;
    pub async fn rollback(self) -> Result<()>;

    // Savepoint operations
    pub async fn savepoint(&mut self, name: &str) -> Result<()>;
    pub async fn rollback_to(&mut self, name: &str) -> Result<()>;
    pub async fn release_savepoint(&mut self, name: &str) -> Result<()>;

    // Execute within transaction
    pub async fn execute(&mut self, query: &str) -> Result<PgQueryResult>;
    pub async fn query<T: Row>(&mut self, builder: &QueryBuilder) -> Result<Vec<T>>;
}
```

**Example Usage**:
```rust
// Simple transaction
let mut tx = conn.begin().await?;

let user = User { id: 1, email: "test@example.com", /* ... */ };
user.insert(&tx).await?;

tx.commit().await?;

// Transaction with savepoints
let mut tx = conn.begin().await?;

// Create user
let user = create_user(&mut tx).await?;
tx.savepoint("after_user").await?;

// Try to create profile
match create_profile(&mut tx, user.id).await {
    Ok(_) => tx.commit().await?,
    Err(_) => {
        // Rollback profile creation but keep user
        tx.rollback_to("after_user").await?;
        tx.commit().await?;
    }
}
```

## 5. Migration Runner (`migration.rs`)

**Role**: Manages database schema migrations with version tracking.

**Key Responsibilities**:
- **Version Control**: Tracks applied migrations in a dedicated table.
- **Up/Down Migrations**: Supports both applying and rolling back migrations.
- **Ordering**: Ensures migrations run in correct order.
- **Schema Introspection**: Can query current database schema state.

**Rust Types**:
```rust
pub struct Migration {
    pub version: i64,
    pub name: String,
    pub up: String,    // SQL for applying migration
    pub down: String,  // SQL for rolling back migration
}

pub struct MigrationRunner {
    conn: Connection,
    migrations_table: String,
}

impl MigrationRunner {
    pub async fn new(conn: Connection) -> Result<Self>;
    pub async fn with_table_name(conn: Connection, table: &str) -> Result<Self>;

    // Migration operations
    pub async fn run_migrations(&self, migrations: Vec<Migration>) -> Result<Vec<i64>>;
    pub async fn rollback(&self, steps: usize) -> Result<Vec<i64>>;
    pub async fn rollback_to(&self, version: i64) -> Result<Vec<i64>>;

    // Status queries
    pub async fn applied_migrations(&self) -> Result<Vec<i64>>;
    pub async fn pending_migrations(&self, all: Vec<Migration>) -> Result<Vec<Migration>>;

    // Schema introspection
    pub async fn table_exists(&self, table: &str) -> Result<bool>;
    pub async fn column_exists(&self, table: &str, column: &str) -> Result<bool>;
    pub async fn get_schema(&self, table: &str) -> Result<TableSchema>;
}

pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
}

pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
}
```

**Example Usage**:
```rust
// Define migrations
let migrations = vec![
    Migration {
        version: 1,
        name: "create_users_table".to_string(),
        up: r#"
            CREATE TABLE users (
                id BIGSERIAL PRIMARY KEY,
                email VARCHAR(255) NOT NULL UNIQUE,
                name VARCHAR(255) NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        "#.to_string(),
        down: "DROP TABLE users".to_string(),
    },
    Migration {
        version: 2,
        name: "add_users_status_column".to_string(),
        up: r#"
            ALTER TABLE users
            ADD COLUMN status VARCHAR(50) NOT NULL DEFAULT 'active'
        "#.to_string(),
        down: "ALTER TABLE users DROP COLUMN status".to_string(),
    },
];

// Run migrations
let runner = MigrationRunner::new(conn).await?;
let applied = runner.run_migrations(migrations).await?;
println!("Applied migrations: {:?}", applied);

// Check pending
let pending = runner.pending_migrations(all_migrations).await?;

// Rollback last migration
runner.rollback(1).await?;

// Schema introspection
if runner.table_exists("users").await? {
    let schema = runner.get_schema("users").await?;
    for col in schema.columns {
        println!("{}: {}", col.name, col.data_type);
    }
}
```

## Component Interaction

```mermaid
graph LR
    User[Python User]

    subgraph "PyO3 Layer"
        PR[PgRow Class]
        QE[Query Engine]
    end

    subgraph "Core Layer"
        Conn[Connection]
        RT[Row Trait]
        QB[Query Builder]
        TX[Transaction]
        MR[Migration Runner]
    end

    User --> PR
    PR -- "extract" --> QE
    QE -- "SQL" --> RT
    RT -- "execute" --> Conn

    User -- "find()" --> QB
    QB -- "execute" --> Conn
    Conn -- "PgRow" --> QE
    QE -- "Python Obj" --> User

    User -- "migrate()" --> MR
    MR -- "execute" --> Conn

    Conn -- "begin()" --> TX
    TX -- "execute" --> Conn
```

## Design Patterns

### 1. Pool-Based Connection Management
Unlike MongoDB's internal pooling, PostgreSQL uses explicit `PgPool` from sqlx with configurable limits.

### 2. Type-Safe Query Building
The query builder prevents SQL injection by:
- Using parameterized queries exclusively
- Validating identifiers (table/column names)
- Separating SQL structure from user data

### 3. Transaction Composition
Transactions can be nested using savepoints, allowing partial rollback without losing all work.

### 4. Schema Evolution
Migration runner provides:
- Forward migrations (up)
- Backward migrations (down)
- Version tracking
- Schema introspection

## Performance Considerations

### Connection Pooling
- **Min connections**: Keep warm connections ready (reduce latency)
- **Max connections**: Prevent PostgreSQL connection exhaustion
- **Idle timeout**: Release unused connections
- **Max lifetime**: Prevent stale connections

### Batch Operations
```rust
// Efficient bulk insert
async fn insert_many<T: Row>(conn: &Connection, rows: Vec<T>) -> Result<Vec<T>> {
    // Use COPY or multi-row INSERT for batches
    // PostgreSQL optimizes multi-row inserts
}
```

### Prepared Statements
sqlx automatically prepares frequently used queries, reducing parsing overhead.

## Next Steps

1. Implement connection pooling with configurable limits
2. Define Row trait with common CRUD operations
3. Build query builder with PostgreSQL-specific operators
4. Add transaction support with savepoint handling
5. Create migration framework with version tracking
