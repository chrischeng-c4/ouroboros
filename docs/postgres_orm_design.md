# PostgreSQL ORM Design: Relationships, Upsert, and Auto-Migration

**Version**: 1.0
**Date**: 2025-12-29
**Status**: Design Document - Implementation Pending

---

## Table of Contents

1. [Overview](#overview)
2. [Relationship API Design](#1-relationship-api-design-python-layer)
3. [Rust Backend Architecture](#2-rust-backend-architecture)
4. [Upsert Implementation](#3-upsert-implementation)
5. [Auto-Migration Generation](#4-auto-migration-generation)
6. [Database Schema Changes](#5-database-schema-changes)
7. [Integration with Existing Features](#6-integration-with-existing-features)
8. [Implementation Roadmap](#7-implementation-roadmap)

---

## Overview

This document outlines the design for three major PostgreSQL ORM features:

1. **Relationships**: ForeignKey and BackReference for modeling table relationships
2. **Upsert**: INSERT ON CONFLICT UPDATE for atomic insert-or-update operations
3. **Auto-Migration**: Automatic migration file generation from schema differences

These features are inspired by:
- MongoDB's Link/BackLink pattern (adapted for SQL)
- Django/SQLAlchemy's relationship system
- Alembic/Django's auto-migration tools

### Design Principles

1. **Type-Safe**: Leverage Python type hints for compile-time checking
2. **Performance-First**: All heavy operations in Rust (JOINs, diffing)
3. **Developer-Friendly**: Intuitive API matching MongoDB's Link/BackLink
4. **PostgreSQL-Native**: Use native SQL features (JOINs, constraints, ON CONFLICT)
5. **Beanie-Compatible**: Similar API patterns where applicable

---

## 1. Relationship API Design (Python Layer)

### 1.1 ForeignKey[T] - Forward Reference

`ForeignKey[T]` represents a reference from the current table to another table (one-to-many or many-to-one).

#### Basic Usage

```python
from data_bridge.postgres import Table, Column, ForeignKey

class User(Table):
    id: int  # Primary key (auto-generated)
    name: str
    email: str = Column(unique=True)

    class Settings:
        table_name = "users"

class Post(Table):
    id: int
    title: str
    content: str
    # Foreign key: stores user_id as integer
    author: ForeignKey[User]

    class Settings:
        table_name = "posts"

# Creates table with:
# CREATE TABLE posts (
#     id SERIAL PRIMARY KEY,
#     title VARCHAR(255) NOT NULL,
#     content TEXT NOT NULL,
#     author_id INTEGER REFERENCES users(id)
# );
```

#### Advanced Configuration

```python
class Post(Table):
    title: str
    author: ForeignKey[User] = ForeignKey(
        # Column name in this table (default: "author_id")
        column_name="author_id",

        # Referenced table/column (auto-detected from type hint)
        referenced_table="users",
        referenced_column="id",

        # Cascade rules
        on_delete="CASCADE",  # DELETE_LINKS, CASCADE, SET_NULL, RESTRICT
        on_update="CASCADE",  # CASCADE, RESTRICT

        # Whether to create database-level foreign key constraint
        constraint=True,  # Default: True

        # Whether to create index on foreign key column
        index=True,  # Default: True

        # Whether foreign key is nullable
        nullable=False,  # Default: False
    )
```

#### Working with ForeignKey

```python
# Create with reference (lazy - stores ID only)
user = await User(name="Alice", email="alice@example.com").save()
post = Post(title="Hello", author=user)  # Stores user.id
await post.save()

# Access foreign key value (ID)
print(post.author.ref)  # 1 (user ID)
print(post.author.id)   # 1 (alias for ref)
print(post.author.column_value)  # 1 (raw column value)

# Check if fetched
print(post.author.is_fetched)  # False

# Fetch the related object (lazy loading)
author = await post.author.fetch()
print(author.name)  # "Alice"
print(post.author.is_fetched)  # True

# Access attributes (auto-fetches if not fetched)
# WARNING: Triggers N+1 queries if used in loops!
print(post.author.name)  # Raises ValueError if not fetched
```

#### Eager Loading

```python
# Fetch posts with authors in a single query (INNER JOIN)
posts = await Post.find().fetch_links("author").to_list()
for post in posts:
    print(post.author.name)  # No additional query

# Multiple relationships
posts = await Post.find().fetch_links("author", "category").to_list()

# LEFT JOIN for nullable foreign keys
posts = await Post.find().fetch_links("author", join_type="left").to_list()
```

### 1.2 BackReference[T] - Reverse Relationship

`BackReference[T]` represents the reverse side of a relationship (one-to-many from the referenced table's perspective).

#### Basic Usage

```python
class User(Table):
    name: str
    # Reverse relationship: find all posts by this user
    posts: BackReference["Post"] = BackReference(
        foreign_key="author"  # Name of ForeignKey field in Post
    )

class Post(Table):
    title: str
    author: ForeignKey[User]

# Usage
user = await User.find_one(User.name == "Alice", fetch_links=True)
for post in user.posts:
    print(post.title)
```

#### Advanced Configuration

```python
class User(Table):
    posts: BackReference["Post"] = BackReference(
        # Name of the ForeignKey field in the related table
        foreign_key="author",

        # Optional: Specify the related table class explicitly
        # (auto-detected from type hint)
        document_class=Post,

        # Optional: Add filters to the reverse query
        filters={"status": "published"},

        # Optional: Ordering
        order_by="-created_at",  # Descending

        # Optional: Limit number of results
        limit=10,
    )
```

#### Working with BackReference

```python
# Fetch with back references
user = await User.find_one(User.id == 1, fetch_links=True)

# Access back reference (returns list)
for post in user.posts:
    print(post.title)

# Check if fetched
print(user.posts.is_fetched)  # True

# Manual fetch (if not auto-fetched)
await user.posts.fetch()

# Count related items without fetching all
count = await user.posts.count()

# Iterate with pagination
async for post in user.posts.paginate(page_size=10):
    print(post.title)
```

### 1.3 Cascade Operations

#### WriteRules (Cascade Save)

```python
from data_bridge.postgres import WriteRules

class Post(Table):
    author: ForeignKey[User] = ForeignKey(
        write_rule=WriteRules.WRITE  # Default: DO_NOTHING
    )

# Create unsaved user
user = User(name="Bob", email="bob@example.com")

# Create post referencing unsaved user
post = Post(title="Test", author=user)

# Save with cascade (saves user first, then post)
await post.save(link_rule=WriteRules.WRITE)
print(user.id)  # Now set (user was auto-saved)
```

#### DeleteRules (Cascade Delete)

```python
from data_bridge.postgres import DeleteRules

class User(Table):
    posts: BackReference["Post"] = BackReference(
        foreign_key="author",
        delete_rule=DeleteRules.DELETE_LINKS  # Default: DO_NOTHING
    )

# Delete user and all their posts
user = await User.get(1)
await user.delete(link_rule=DeleteRules.DELETE_LINKS)
# Executes: DELETE FROM posts WHERE author_id = 1
#           DELETE FROM users WHERE id = 1
```

### 1.4 Many-to-Many Relationships

```python
class Student(Table):
    name: str
    courses: ManyToMany["Course"] = ManyToMany(
        through="student_courses",  # Join table name
        foreign_key="student_id",   # FK in join table
        related_key="course_id",    # Related FK in join table
    )

class Course(Table):
    title: str
    students: ManyToMany["Student"] = ManyToMany(
        through="student_courses",
        foreign_key="course_id",
        related_key="student_id",
    )

# Creates join table automatically:
# CREATE TABLE student_courses (
#     id SERIAL PRIMARY KEY,
#     student_id INTEGER REFERENCES students(id) ON DELETE CASCADE,
#     course_id INTEGER REFERENCES courses(id) ON DELETE CASCADE,
#     UNIQUE (student_id, course_id)
# );

# Usage
student = await Student.get(1)
await student.courses.add(course1, course2)  # Insert into join table
await student.courses.remove(course1)        # Delete from join table
await student.courses.clear()                # Delete all associations

# Fetch with many-to-many
student = await Student.find_one(Student.id == 1, fetch_links=True)
for course in student.courses:
    print(course.title)
```

---

## 2. Rust Backend Architecture

### 2.1 Schema System Extensions

#### Foreign Key Tracking

```rust
// crates/data-bridge-postgres/src/schema.rs

/// Represents a foreign key relationship in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyConfig {
    /// Column name in the source table (e.g., "author_id")
    pub column_name: String,

    /// Referenced table name (e.g., "users")
    pub referenced_table: String,

    /// Referenced column name (e.g., "id")
    pub referenced_column: String,

    /// ON DELETE action
    pub on_delete: CascadeAction,

    /// ON UPDATE action
    pub on_update: CascadeAction,

    /// Whether to create database constraint
    pub constraint: bool,

    /// Whether to create index on foreign key column
    pub index: bool,

    /// Whether the foreign key is nullable
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CascadeAction {
    Cascade,
    SetNull,
    Restrict,
    NoAction,
}

/// Extended table schema with relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub schema: String,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub foreign_keys: Vec<ForeignKeyConfig>,
    pub primary_key: Vec<String>,
}
```

#### Schema Builder

```rust
// crates/data-bridge-postgres/src/schema_builder.rs

pub struct SchemaBuilder {
    table_name: String,
    schema: String,
    columns: Vec<ColumnDef>,
    foreign_keys: Vec<ForeignKeyConfig>,
    indexes: Vec<IndexConfig>,
    primary_key: Vec<String>,
}

impl SchemaBuilder {
    pub fn add_foreign_key(&mut self, fk: ForeignKeyConfig) -> &mut Self {
        // Add foreign key column if not exists
        if !self.columns.iter().any(|c| c.name == fk.column_name) {
            self.columns.push(ColumnDef {
                name: fk.column_name.clone(),
                data_type: ColumnType::Integer,  // or BigInt
                nullable: fk.nullable,
                default: None,
            });
        }

        // Add index if requested
        if fk.index {
            self.indexes.push(IndexConfig {
                columns: vec![fk.column_name.clone()],
                unique: false,
                index_type: IndexType::BTree,
            });
        }

        self.foreign_keys.push(fk);
        self
    }

    pub fn build_sql(&self) -> String {
        // Generate CREATE TABLE with foreign key constraints
        let mut sql = format!("CREATE TABLE {}.{} (\n", self.schema, self.table_name);

        // Columns
        for col in &self.columns {
            sql.push_str(&format!("  {} {},\n", col.name, col.data_type_sql()));
        }

        // Primary key
        if !self.primary_key.is_empty() {
            sql.push_str(&format!("  PRIMARY KEY ({}),\n", self.primary_key.join(", ")));
        }

        // Foreign keys
        for fk in &self.foreign_keys {
            if fk.constraint {
                sql.push_str(&format!(
                    "  CONSTRAINT fk_{}_{} FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE {} ON UPDATE {},\n",
                    self.table_name, fk.column_name,
                    fk.column_name,
                    fk.referenced_table,
                    fk.referenced_column,
                    fk.on_delete.to_sql(),
                    fk.on_update.to_sql()
                ));
            }
        }

        sql.pop(); // Remove trailing comma
        sql.pop();
        sql.push_str("\n);");
        sql
    }
}
```

### 2.2 JOIN Query Generation

#### Query Builder Extensions

```rust
// crates/data-bridge-postgres/src/query_builder.rs

pub struct QueryBuilder {
    table: String,
    schema: String,
    select_columns: Vec<String>,
    where_clause: Option<String>,
    joins: Vec<JoinClause>,
    order_by: Vec<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub schema: String,
    pub on_condition: String,
    /// Column aliases: (source_column, alias)
    pub column_aliases: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl QueryBuilder {
    pub fn join(
        &mut self,
        foreign_key: &ForeignKeyConfig,
        join_type: JoinType,
    ) -> &mut Self {
        let join = JoinClause {
            join_type,
            table: foreign_key.referenced_table.clone(),
            schema: self.schema.clone(),
            on_condition: format!(
                "{}.{} = {}.{}",
                self.table,
                foreign_key.column_name,
                foreign_key.referenced_table,
                foreign_key.referenced_column
            ),
            column_aliases: vec![],  // Populated based on referenced table schema
        };

        self.joins.push(join);
        self
    }

    pub fn build(&self) -> String {
        let mut sql = String::from("SELECT ");

        // Select columns with table prefixes
        if self.select_columns.is_empty() {
            sql.push_str(&format!("{}.*", self.table));
        } else {
            sql.push_str(&self.select_columns.join(", "));
        }

        // Add joined table columns with aliases
        for join in &self.joins {
            for (col, alias) in &join.column_aliases {
                sql.push_str(&format!(", {}.{} AS {}", join.table, col, alias));
            }
        }

        // FROM clause
        sql.push_str(&format!("\nFROM {}.{}", self.schema, self.table));

        // JOIN clauses
        for join in &self.joins {
            sql.push_str(&format!(
                "\n{} JOIN {}.{} ON {}",
                join.join_type.to_sql(),
                join.schema,
                join.table,
                join.on_condition
            ));
        }

        // WHERE clause
        if let Some(where_clause) = &self.where_clause {
            sql.push_str(&format!("\nWHERE {}", where_clause));
        }

        // ORDER BY
        if !self.order_by.is_empty() {
            sql.push_str(&format!("\nORDER BY {}", self.order_by.join(", ")));
        }

        // LIMIT/OFFSET
        if let Some(limit) = self.limit {
            sql.push_str(&format!("\nLIMIT {}", limit));
        }
        if let Some(offset) = self.offset {
            sql.push_str(&format!("\nOFFSET {}", offset));
        }

        sql
    }
}
```

### 2.3 Relationship Resolution

#### Lazy Loading (Fetch on Demand)

```rust
// crates/data-bridge-postgres/src/relationships.rs

pub struct RelationshipResolver {
    conn: Connection,
}

impl RelationshipResolver {
    /// Fetch a single related object by foreign key.
    pub async fn fetch_foreign_key<T>(
        &self,
        foreign_key_value: i64,
        referenced_table: &str,
        referenced_column: &str,
    ) -> Result<Option<HashMap<String, serde_json::Value>>> {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = $1",
            referenced_table,
            referenced_column
        );

        let row = sqlx::query(&sql)
            .bind(foreign_key_value)
            .fetch_optional(self.conn.pool())
            .await?;

        if let Some(row) = row {
            Ok(Some(row_to_hashmap(row)?))
        } else {
            Ok(None)
        }
    }

    /// Fetch related objects for a back reference (reverse foreign key).
    pub async fn fetch_back_reference<T>(
        &self,
        parent_id: i64,
        related_table: &str,
        foreign_key_column: &str,
    ) -> Result<Vec<HashMap<String, serde_json::Value>>> {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = $1",
            related_table,
            foreign_key_column
        );

        let rows = sqlx::query(&sql)
            .bind(parent_id)
            .fetch_all(self.conn.pool())
            .await?;

        rows.into_iter()
            .map(row_to_hashmap)
            .collect()
    }
}
```

#### Eager Loading (JOIN-based)

```rust
impl RelationshipResolver {
    /// Fetch multiple objects with their related objects in one query.
    pub async fn fetch_with_joins(
        &self,
        query_builder: &QueryBuilder,
        relationship_configs: &[RelationshipConfig],
    ) -> Result<Vec<RelatedRow>> {
        // Build JOIN query
        let sql = query_builder.build();

        // Execute query
        let rows = sqlx::query(&sql)
            .fetch_all(self.conn.pool())
            .await?;

        // Parse results into nested structures
        let mut results = Vec::new();
        for row in rows {
            let related_row = self.parse_joined_row(row, relationship_configs)?;
            results.push(related_row);
        }

        Ok(results)
    }

    fn parse_joined_row(
        &self,
        row: PgRow,
        configs: &[RelationshipConfig],
    ) -> Result<RelatedRow> {
        let mut main_data = HashMap::new();
        let mut related_data = HashMap::new();

        // Extract main table columns
        for col in row.columns() {
            let col_name = col.name();

            // Check if this is a joined column (has alias)
            if let Some(config) = configs.iter().find(|c| col_name.starts_with(&c.alias_prefix)) {
                // This is a related column
                let related_col_name = col_name.strip_prefix(&config.alias_prefix).unwrap();
                related_data
                    .entry(&config.field_name)
                    .or_insert_with(HashMap::new)
                    .insert(related_col_name.to_string(), get_column_value(&row, col_name)?);
            } else {
                // Main table column
                main_data.insert(col_name.to_string(), get_column_value(&row, col_name)?);
            }
        }

        Ok(RelatedRow {
            main: main_data,
            related: related_data,
        })
    }
}
```

### 2.4 Cascade Operations

```rust
// crates/data-bridge-postgres/src/cascade.rs

pub struct CascadeHandler {
    conn: Connection,
}

impl CascadeHandler {
    /// Cascade save: Save related objects before saving the main object.
    pub async fn cascade_save(
        &self,
        main_table: &str,
        main_data: &HashMap<String, serde_json::Value>,
        foreign_keys: &[ForeignKeySaveConfig],
    ) -> Result<HashMap<String, i64>> {
        let mut foreign_key_values = HashMap::new();

        for fk in foreign_keys {
            if let Some(related_object) = main_data.get(&fk.field_name) {
                // Check if the related object needs to be saved
                if related_object.get("id").is_none() {
                    // Insert the related object first
                    let related_id = self.insert_related(
                        &fk.referenced_table,
                        related_object,
                    ).await?;

                    foreign_key_values.insert(fk.column_name.clone(), related_id);
                } else {
                    // Related object already has an ID
                    let related_id = related_object["id"].as_i64().unwrap();
                    foreign_key_values.insert(fk.column_name.clone(), related_id);
                }
            }
        }

        Ok(foreign_key_values)
    }

    /// Cascade delete: Delete related objects when deleting the main object.
    pub async fn cascade_delete(
        &self,
        main_table: &str,
        main_id: i64,
        back_references: &[BackReferenceConfig],
    ) -> Result<u64> {
        let mut total_deleted = 0;

        for back_ref in back_references {
            // Delete all objects that reference this one
            let sql = format!(
                "DELETE FROM {} WHERE {} = $1",
                back_ref.related_table,
                back_ref.foreign_key_column
            );

            let result = sqlx::query(&sql)
                .bind(main_id)
                .execute(self.conn.pool())
                .await?;

            total_deleted += result.rows_affected();
        }

        Ok(total_deleted)
    }
}
```

---

## 3. Upsert Implementation

### 3.1 Python API

```python
from data_bridge.postgres import Table, Column

class User(Table):
    email: str = Column(unique=True)
    name: str
    age: int

# Basic upsert
await User.upsert(
    {"email": "alice@example.com", "name": "Alice", "age": 30},
    conflict_target="email",
)
# SQL: INSERT INTO users (email, name, age) VALUES ($1, $2, $3)
#      ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name, age = EXCLUDED.age

# Selective update (only update specific columns on conflict)
await User.upsert(
    {"email": "alice@example.com", "name": "Alice Updated", "age": 31},
    conflict_target="email",
    update_columns=["name"],  # Only update name, not age
)
# SQL: ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name

# Multiple conflict targets (composite unique constraint)
await User.upsert(
    {"email": "alice@example.com", "department": "Engineering", "name": "Alice"},
    conflict_target=["email", "department"],
)

# Do nothing on conflict (only insert if not exists)
await User.upsert(
    {"email": "alice@example.com", "name": "Alice"},
    conflict_target="email",
    on_conflict="do_nothing",
)
# SQL: ON CONFLICT (email) DO NOTHING

# Upsert many
await User.upsert_many([
    {"email": "alice@example.com", "name": "Alice", "age": 30},
    {"email": "bob@example.com", "name": "Bob", "age": 25},
], conflict_target="email")
```

### 3.2 Instance Method

```python
# Upsert on instance
user = User(email="alice@example.com", name="Alice", age=30)
await user.upsert(conflict_target="email")

# Equivalent to:
# IF EXISTS (SELECT 1 FROM users WHERE email = 'alice@example.com')
#   UPDATE ...
# ELSE
#   INSERT ...
```

### 3.3 Rust Implementation

```rust
// crates/data-bridge-postgres/src/upsert.rs

pub struct UpsertBuilder {
    table: String,
    schema: String,
    data: HashMap<String, serde_json::Value>,
    conflict_target: ConflictTarget,
    on_conflict_action: OnConflictAction,
}

#[derive(Debug, Clone)]
pub enum ConflictTarget {
    /// Single column: ON CONFLICT (email)
    Column(String),

    /// Multiple columns: ON CONFLICT (email, department)
    Columns(Vec<String>),

    /// Named constraint: ON CONFLICT ON CONSTRAINT users_email_key
    Constraint(String),
}

#[derive(Debug, Clone)]
pub enum OnConflictAction {
    /// Do nothing on conflict
    DoNothing,

    /// Update all columns (except primary key and conflict columns)
    UpdateAll,

    /// Update specific columns
    UpdateColumns(Vec<String>),
}

impl UpsertBuilder {
    pub fn build(&self) -> (String, Vec<serde_json::Value>) {
        let columns: Vec<String> = self.data.keys().cloned().collect();
        let placeholders: Vec<String> = (1..=columns.len())
            .map(|i| format!("${}", i))
            .collect();

        let mut sql = format!(
            "INSERT INTO {}.{} ({}) VALUES ({})",
            self.schema,
            self.table,
            columns.join(", "),
            placeholders.join(", ")
        );

        // ON CONFLICT clause
        sql.push_str(" ON CONFLICT ");
        match &self.conflict_target {
            ConflictTarget::Column(col) => {
                sql.push_str(&format!("({})", col));
            }
            ConflictTarget::Columns(cols) => {
                sql.push_str(&format!("({})", cols.join(", ")));
            }
            ConflictTarget::Constraint(name) => {
                sql.push_str(&format!("ON CONSTRAINT {}", name));
            }
        }

        // DO UPDATE or DO NOTHING
        match &self.on_conflict_action {
            OnConflictAction::DoNothing => {
                sql.push_str(" DO NOTHING");
            }
            OnConflictAction::UpdateAll => {
                let updates: Vec<String> = columns
                    .iter()
                    .filter(|c| !self.is_conflict_column(c))
                    .map(|c| format!("{} = EXCLUDED.{}", c, c))
                    .collect();

                if !updates.is_empty() {
                    sql.push_str(&format!(" DO UPDATE SET {}", updates.join(", ")));
                }
            }
            OnConflictAction::UpdateColumns(update_cols) => {
                let updates: Vec<String> = update_cols
                    .iter()
                    .map(|c| format!("{} = EXCLUDED.{}", c, c))
                    .collect();

                sql.push_str(&format!(" DO UPDATE SET {}", updates.join(", ")));
            }
        }

        sql.push_str(" RETURNING id");

        let values: Vec<serde_json::Value> = columns
            .iter()
            .map(|c| self.data[c].clone())
            .collect();

        (sql, values)
    }

    fn is_conflict_column(&self, column: &str) -> bool {
        match &self.conflict_target {
            ConflictTarget::Column(col) => column == col,
            ConflictTarget::Columns(cols) => cols.contains(&column.to_string()),
            ConflictTarget::Constraint(_) => false,  // Let PostgreSQL handle it
        }
    }
}

pub async fn upsert_one(
    conn: &Connection,
    table: &str,
    data: HashMap<String, serde_json::Value>,
    conflict_target: ConflictTarget,
    on_conflict: OnConflictAction,
) -> Result<i64> {
    let builder = UpsertBuilder {
        table: table.to_string(),
        schema: "public".to_string(),
        data,
        conflict_target,
        on_conflict_action: on_conflict,
    };

    let (sql, values) = builder.build();

    // Execute query with parameters
    let mut query = sqlx::query(&sql);
    for value in values {
        query = query.bind(value);
    }

    let row = query.fetch_one(conn.pool()).await?;
    let id: i64 = row.try_get("id")?;

    Ok(id)
}
```

### 3.4 Batch Upsert with Performance Optimization

```rust
pub async fn upsert_many(
    conn: &Connection,
    table: &str,
    rows: Vec<HashMap<String, serde_json::Value>>,
    conflict_target: ConflictTarget,
    on_conflict: OnConflictAction,
) -> Result<Vec<i64>> {
    if rows.is_empty() {
        return Ok(vec![]);
    }

    // Build multi-row INSERT
    let columns: Vec<String> = rows[0].keys().cloned().collect();
    let num_cols = columns.len();

    let mut placeholders = Vec::new();
    let mut all_values = Vec::new();

    for (row_idx, row) in rows.iter().enumerate() {
        let row_placeholders: Vec<String> = (0..num_cols)
            .map(|col_idx| {
                let param_idx = row_idx * num_cols + col_idx + 1;
                format!("${}", param_idx)
            })
            .collect();

        placeholders.push(format!("({})", row_placeholders.join(", ")));

        for col in &columns {
            all_values.push(row[col].clone());
        }
    }

    let mut sql = format!(
        "INSERT INTO {} ({}) VALUES {}",
        table,
        columns.join(", "),
        placeholders.join(", ")
    );

    // Add ON CONFLICT clause
    sql.push_str(&build_on_conflict_clause(&conflict_target, &on_conflict, &columns));
    sql.push_str(" RETURNING id");

    // Execute query
    let mut query = sqlx::query(&sql);
    for value in all_values {
        query = query.bind(value);
    }

    let rows = query.fetch_all(conn.pool()).await?;
    let ids: Vec<i64> = rows.iter().map(|r| r.try_get("id").unwrap()).collect();

    Ok(ids)
}
```

---

## 4. Auto-Migration Generation

### 4.1 Python API

```python
from data_bridge.postgres import migration

# Generate migration by comparing ORM models to database schema
migration_file = await migration.generate(
    "add_user_status_column",
    migrations_dir="migrations",
)

# Output:
# Created: migrations/20250129_150000_add_user_status_column.sql
#
# --UP
# ALTER TABLE users ADD COLUMN status VARCHAR(50) NOT NULL DEFAULT 'active';
# CREATE INDEX idx_users_status ON users(status);
#
# --DOWN
# DROP INDEX idx_users_status;
# ALTER TABLE users DROP COLUMN status;

# Apply generated migrations
await migration.apply(migrations_dir="migrations")

# Rollback
await migration.rollback(migrations_dir="migrations", steps=1)
```

### 4.2 Schema Comparison

```rust
// crates/data-bridge-postgres/src/migration_generator.rs

pub struct SchemaDiff {
    pub new_tables: Vec<TableSchema>,
    pub dropped_tables: Vec<String>,
    pub modified_tables: Vec<TableModification>,
}

#[derive(Debug, Clone)]
pub struct TableModification {
    pub table_name: String,
    pub new_columns: Vec<ColumnInfo>,
    pub dropped_columns: Vec<String>,
    pub modified_columns: Vec<ColumnModification>,
    pub new_indexes: Vec<IndexInfo>,
    pub dropped_indexes: Vec<String>,
    pub new_foreign_keys: Vec<ForeignKeyInfo>,
    pub dropped_foreign_keys: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ColumnModification {
    pub column_name: String,
    pub changes: Vec<ColumnChange>,
}

#[derive(Debug, Clone)]
pub enum ColumnChange {
    TypeChange { from: ColumnType, to: ColumnType },
    NullableChange { from: bool, to: bool },
    DefaultChange { from: Option<String>, to: Option<String> },
}

pub struct MigrationGenerator {
    conn: Connection,
}

impl MigrationGenerator {
    /// Compare ORM schema definitions to current database schema.
    pub async fn compare_schemas(
        &self,
        orm_schemas: Vec<TableSchema>,
    ) -> Result<SchemaDiff> {
        let inspector = SchemaInspector::new(self.conn.clone());

        // Get current database tables
        let db_tables = inspector.list_tables(Some("public")).await?;

        let mut new_tables = Vec::new();
        let mut dropped_tables = Vec::new();
        let mut modified_tables = Vec::new();

        // Find new tables
        for orm_table in &orm_schemas {
            if !db_tables.contains(&orm_table.name) {
                new_tables.push(orm_table.clone());
            } else {
                // Table exists, check for modifications
                let db_schema = inspector.inspect_table(&orm_table.name, Some("public")).await?;

                if let Some(modification) = self.compare_table_schemas(orm_table, &db_schema) {
                    modified_tables.push(modification);
                }
            }
        }

        // Find dropped tables
        for db_table in &db_tables {
            if !orm_schemas.iter().any(|t| &t.name == db_table) {
                dropped_tables.push(db_table.clone());
            }
        }

        Ok(SchemaDiff {
            new_tables,
            dropped_tables,
            modified_tables,
        })
    }

    fn compare_table_schemas(
        &self,
        orm_schema: &TableSchema,
        db_schema: &TableInfo,
    ) -> Option<TableModification> {
        let mut new_columns = Vec::new();
        let mut dropped_columns = Vec::new();
        let mut modified_columns = Vec::new();

        // Compare columns
        for orm_col in &orm_schema.columns {
            if let Some(db_col) = db_schema.columns.iter().find(|c| c.name == orm_col.name) {
                // Column exists, check for modifications
                let changes = self.compare_columns(orm_col, db_col);
                if !changes.is_empty() {
                    modified_columns.push(ColumnModification {
                        column_name: orm_col.name.clone(),
                        changes,
                    });
                }
            } else {
                // New column
                new_columns.push(orm_col.clone());
            }
        }

        // Find dropped columns
        for db_col in &db_schema.columns {
            if !orm_schema.columns.iter().any(|c| c.name == db_col.name) {
                dropped_columns.push(db_col.name.clone());
            }
        }

        // Compare indexes
        let new_indexes = self.find_new_indexes(&orm_schema.indexes, &db_schema.indexes);
        let dropped_indexes = self.find_dropped_indexes(&orm_schema.indexes, &db_schema.indexes);

        // Compare foreign keys
        let new_foreign_keys = self.find_new_foreign_keys(
            &orm_schema.foreign_keys,
            &db_schema.foreign_keys,
        );
        let dropped_foreign_keys = self.find_dropped_foreign_keys(
            &orm_schema.foreign_keys,
            &db_schema.foreign_keys,
        );

        // Return modification if there are any changes
        if new_columns.is_empty()
            && dropped_columns.is_empty()
            && modified_columns.is_empty()
            && new_indexes.is_empty()
            && dropped_indexes.is_empty()
            && new_foreign_keys.is_empty()
            && dropped_foreign_keys.is_empty()
        {
            None
        } else {
            Some(TableModification {
                table_name: orm_schema.name.clone(),
                new_columns,
                dropped_columns,
                modified_columns,
                new_indexes,
                dropped_indexes,
                new_foreign_keys,
                dropped_foreign_keys,
            })
        }
    }

    fn compare_columns(&self, orm_col: &ColumnInfo, db_col: &ColumnInfo) -> Vec<ColumnChange> {
        let mut changes = Vec::new();

        // Type change
        if orm_col.data_type != db_col.data_type {
            changes.push(ColumnChange::TypeChange {
                from: db_col.data_type.clone(),
                to: orm_col.data_type.clone(),
            });
        }

        // Nullable change
        if orm_col.nullable != db_col.nullable {
            changes.push(ColumnChange::NullableChange {
                from: db_col.nullable,
                to: orm_col.nullable,
            });
        }

        // Default change
        if orm_col.default != db_col.default {
            changes.push(ColumnChange::DefaultChange {
                from: db_col.default.clone(),
                to: orm_col.default.clone(),
            });
        }

        changes
    }
}
```

### 4.3 Migration File Generation

```rust
impl MigrationGenerator {
    /// Generate SQL migration file from schema diff.
    pub fn generate_migration_sql(&self, diff: &SchemaDiff) -> (String, String) {
        let mut up_sql = String::new();
        let mut down_sql = String::new();

        // New tables
        for table in &diff.new_tables {
            up_sql.push_str(&format!("-- Create table {}\n", table.name));
            up_sql.push_str(&self.generate_create_table_sql(table));
            up_sql.push_str("\n\n");

            down_sql.push_str(&format!("DROP TABLE IF EXISTS {};\n\n", table.name));
        }

        // Dropped tables
        for table in &diff.dropped_tables {
            up_sql.push_str(&format!("DROP TABLE IF EXISTS {};\n\n", table));

            // Cannot reverse without schema
            down_sql.push_str(&format!("-- TODO: Recreate table {}\n\n", table));
        }

        // Modified tables
        for modification in &diff.modified_tables {
            let (up, down) = self.generate_table_modification_sql(modification);
            up_sql.push_str(&up);
            down_sql.push_str(&down);
        }

        (up_sql, down_sql)
    }

    fn generate_table_modification_sql(&self, mod: &TableModification) -> (String, String) {
        let mut up_sql = String::new();
        let mut down_sql = String::new();

        up_sql.push_str(&format!("-- Modify table {}\n", mod.table_name));

        // New columns
        for col in &mod.new_columns {
            up_sql.push_str(&format!(
                "ALTER TABLE {} ADD COLUMN {} {}{}{};\n",
                mod.table_name,
                col.name,
                col.data_type.to_sql(),
                if col.nullable { "" } else { " NOT NULL" },
                if let Some(default) = &col.default {
                    format!(" DEFAULT {}", default)
                } else {
                    String::new()
                }
            ));

            down_sql.push_str(&format!(
                "ALTER TABLE {} DROP COLUMN {};\n",
                mod.table_name, col.name
            ));
        }

        // Dropped columns
        for col_name in &mod.dropped_columns {
            up_sql.push_str(&format!(
                "ALTER TABLE {} DROP COLUMN {};\n",
                mod.table_name, col_name
            ));

            down_sql.push_str(&format!(
                "-- TODO: Recreate column {}.{}\n",
                mod.table_name, col_name
            ));
        }

        // Modified columns
        for col_mod in &mod.modified_columns {
            for change in &col_mod.changes {
                match change {
                    ColumnChange::TypeChange { from, to } => {
                        up_sql.push_str(&format!(
                            "ALTER TABLE {} ALTER COLUMN {} TYPE {};\n",
                            mod.table_name, col_mod.column_name, to.to_sql()
                        ));

                        down_sql.push_str(&format!(
                            "ALTER TABLE {} ALTER COLUMN {} TYPE {};\n",
                            mod.table_name, col_mod.column_name, from.to_sql()
                        ));
                    }
                    ColumnChange::NullableChange { from, to } => {
                        if *to {
                            up_sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;\n",
                                mod.table_name, col_mod.column_name
                            ));
                        } else {
                            up_sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL;\n",
                                mod.table_name, col_mod.column_name
                            ));
                        }

                        // Reverse
                        if *from {
                            down_sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;\n",
                                mod.table_name, col_mod.column_name
                            ));
                        } else {
                            down_sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL;\n",
                                mod.table_name, col_mod.column_name
                            ));
                        }
                    }
                    ColumnChange::DefaultChange { from, to } => {
                        if let Some(default) = to {
                            up_sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};\n",
                                mod.table_name, col_mod.column_name, default
                            ));
                        } else {
                            up_sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;\n",
                                mod.table_name, col_mod.column_name
                            ));
                        }

                        // Reverse
                        if let Some(default) = from {
                            down_sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};\n",
                                mod.table_name, col_mod.column_name, default
                            ));
                        } else {
                            down_sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;\n",
                                mod.table_name, col_mod.column_name
                            ));
                        }
                    }
                }
            }
        }

        // New indexes
        for idx in &mod.new_indexes {
            up_sql.push_str(&format!(
                "CREATE{}INDEX {} ON {} ({});\n",
                if idx.is_unique { " UNIQUE " } else { " " },
                idx.name,
                mod.table_name,
                idx.columns.join(", ")
            ));

            down_sql.push_str(&format!("DROP INDEX {};\n", idx.name));
        }

        // Dropped indexes
        for idx_name in &mod.dropped_indexes {
            up_sql.push_str(&format!("DROP INDEX {};\n", idx_name));
            down_sql.push_str(&format!("-- TODO: Recreate index {}\n", idx_name));
        }

        // New foreign keys
        for fk in &mod.new_foreign_keys {
            up_sql.push_str(&format!(
                "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE {} ON UPDATE {};\n",
                mod.table_name,
                fk.name,
                fk.columns.join(", "),
                fk.referenced_table,
                fk.referenced_columns.join(", "),
                fk.on_delete,
                fk.on_update
            ));

            down_sql.push_str(&format!(
                "ALTER TABLE {} DROP CONSTRAINT {};\n",
                mod.table_name, fk.name
            ));
        }

        // Dropped foreign keys
        for fk_name in &mod.dropped_foreign_keys {
            up_sql.push_str(&format!(
                "ALTER TABLE {} DROP CONSTRAINT {};\n",
                mod.table_name, fk_name
            ));

            down_sql.push_str(&format!("-- TODO: Recreate foreign key {}\n", fk_name));
        }

        up_sql.push('\n');
        down_sql.push('\n');

        (up_sql, down_sql)
    }
}
```

### 4.4 Migration File Format

```sql
-- Migration: add_user_status_column
-- Created: 2025-12-29 15:00:00
-- Description: Add status column to users table

--UP
ALTER TABLE users ADD COLUMN status VARCHAR(50) NOT NULL DEFAULT 'active';
CREATE INDEX idx_users_status ON users(status);

--DOWN
DROP INDEX idx_users_status;
ALTER TABLE users DROP COLUMN status;
```

---

## 5. Database Schema Changes

### 5.1 Foreign Key Storage

Foreign keys are stored as regular integer columns with optional database-level constraints:

```sql
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,

    -- Foreign key column
    author_id INTEGER NOT NULL,

    -- Foreign key constraint (optional, controlled by constraint=True)
    CONSTRAINT fk_posts_author FOREIGN KEY (author_id)
        REFERENCES users(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE
);

-- Index on foreign key (for efficient JOINs)
CREATE INDEX idx_posts_author_id ON posts(author_id);
```

### 5.2 Many-to-Many Join Tables

```sql
CREATE TABLE student_courses (
    id SERIAL PRIMARY KEY,
    student_id INTEGER NOT NULL,
    course_id INTEGER NOT NULL,

    -- Composite unique constraint (prevent duplicate associations)
    CONSTRAINT uq_student_courses UNIQUE (student_id, course_id),

    -- Foreign key constraints
    CONSTRAINT fk_student_courses_student FOREIGN KEY (student_id)
        REFERENCES students(id) ON DELETE CASCADE,
    CONSTRAINT fk_student_courses_course FOREIGN KEY (course_id)
        REFERENCES courses(id) ON DELETE CASCADE
);

-- Indexes for efficient lookups in both directions
CREATE INDEX idx_student_courses_student ON student_courses(student_id);
CREATE INDEX idx_student_courses_course ON student_courses(course_id);
```

### 5.3 Referential Integrity

PostgreSQL enforces referential integrity at the database level:

1. **ON DELETE CASCADE**: Automatically delete related rows
2. **ON DELETE SET NULL**: Set foreign key to NULL when referenced row is deleted
3. **ON DELETE RESTRICT**: Prevent deletion if related rows exist
4. **ON UPDATE CASCADE**: Update foreign key when referenced primary key changes

```sql
-- Example: Delete user automatically deletes all their posts
CONSTRAINT fk_posts_author FOREIGN KEY (author_id)
    REFERENCES users(id) ON DELETE CASCADE

-- Example: Prevent deletion if posts exist
CONSTRAINT fk_posts_author FOREIGN KEY (author_id)
    REFERENCES users(id) ON DELETE RESTRICT
```

---

## 6. Integration with Existing Features

### 6.1 Table/Column System

Foreign keys integrate seamlessly with existing `Table` and `Column`:

```python
class Post(Table):
    title: str = Column(nullable=False)
    content: str
    # ForeignKey is a special kind of Column
    author: ForeignKey[User] = ForeignKey(on_delete="CASCADE")

    class Settings:
        table_name = "posts"
        indexes = [
            {"columns": ["author_id"]},  # Explicit index
        ]
```

Internally, `ForeignKey[User]` creates:
- A regular `Column` named `author_id` (integer)
- Metadata for relationship tracking
- Optional database constraint

### 6.2 Migration System

The auto-migration generator builds on the existing migration system:

```python
# Existing: Manual migration creation
migration_create("add_status_column", migrations_dir="migrations")

# New: Auto-generate from schema diff
await migration.generate("add_status_column", migrations_dir="migrations")

# Existing migration apply/rollback unchanged
await migration.apply(migrations_dir="migrations")
await migration.rollback(migrations_dir="migrations", steps=1)
```

Migration files use the same format:

```
migrations/
├── 20250101_000000_create_users.sql      # Manual
├── 20250102_000000_create_posts.sql      # Manual
└── 20250129_150000_add_status.sql        # Auto-generated
```

### 6.3 Transaction Support

Relationships and upserts work within transactions:

```python
from data_bridge.postgres import begin_transaction, WriteRules

async with begin_transaction() as tx:
    # Create user
    user = User(name="Alice", email="alice@example.com")
    await user.save()

    # Create post with foreign key (cascade save)
    post = Post(title="Hello", author=user)
    await post.save(link_rule=WriteRules.WRITE)

    # Upsert
    await User.upsert(
        {"email": "bob@example.com", "name": "Bob"},
        conflict_target="email"
    )

    # All operations committed together
    await tx.commit()
```

### 6.4 Query Builder

Relationships extend the existing query builder:

```python
# Existing: Simple queries
users = await User.find(User.age > 25).to_list()

# New: Queries with joins
posts = await Post.find(Post.title.like("Python%")).fetch_links("author").to_list()

# Combined
posts = await Post.find(
    Post.title.like("Python%"),
    Post.author.name == "Alice"  # Join filter
).fetch_links("author").to_list()
```

---

## 7. Implementation Roadmap

### Phase 1: ForeignKey Storage and Basic Resolution (2-3 weeks)

**Goal**: Basic foreign key support without JOINs

**Tasks**:
1. Python `ForeignKey[T]` class (similar to `Link[T]`)
   - Store integer ID
   - Lazy fetch via separate query
   - Type hints and validation
2. Rust schema system updates
   - `ForeignKeyConfig` struct
   - Foreign key tracking in `TableSchema`
3. Migration support for foreign keys
   - Generate `ALTER TABLE ADD CONSTRAINT` SQL
   - Support `ON DELETE/UPDATE` actions
4. Basic tests
   - Create/save with foreign keys
   - Lazy fetch
   - Cascade delete (database-level)

**Deliverables**:
- `ForeignKey[T]` class in `python/data_bridge/postgres/columns.py`
- Schema updates in `crates/data-bridge-postgres/src/schema.rs`
- Tests in `tests/postgres/unit/test_foreign_keys.py`

**Complexity**: Medium
**Dependencies**: None

---

### Phase 2: JOIN Support and Eager Loading (3-4 weeks)

**Goal**: Efficient eager loading with JOINs

**Tasks**:
1. Rust `QueryBuilder` extensions
   - `JoinClause` struct
   - JOIN SQL generation (INNER, LEFT, RIGHT)
   - Column aliasing for joined tables
2. Python `fetch_links()` API
   - Parse relationship names
   - Build JOIN queries via Rust
   - Nest joined data in results
3. Result parsing
   - Flatten joined columns back to nested objects
   - Handle NULL values (LEFT JOIN)
4. Performance optimization
   - Single query for N objects with M relationships
   - Avoid N+1 queries

**Deliverables**:
- `QueryBuilder::join()` in `crates/data-bridge-postgres/src/query_builder.rs`
- `fetch_links()` in `python/data_bridge/postgres/query.py`
- Integration tests with benchmarks

**Complexity**: High
**Dependencies**: Phase 1

---

### Phase 3: BackReference and Cascade Operations (2-3 weeks)

**Goal**: Reverse relationships and cascade save/delete

**Tasks**:
1. Python `BackReference[T]` class
   - Similar to `BackLink[T]` but for SQL
   - Reverse query generation
   - Lazy/eager fetch support
2. Rust cascade handlers
   - `CascadeHandler` struct
   - Cascade save (save related objects first)
   - Cascade delete (delete related objects)
3. `WriteRules` and `DeleteRules` enums
   - Integration with `save()` and `delete()`
4. Transaction safety
   - All cascades within same transaction
   - Rollback on error

**Deliverables**:
- `BackReference[T]` in `python/data_bridge/postgres/columns.py`
- `CascadeHandler` in `crates/data-bridge-postgres/src/cascade.rs`
- Cascade tests with transaction safety

**Complexity**: Medium
**Dependencies**: Phase 1, Phase 2

---

### Phase 4: Upsert Implementation (1-2 weeks)

**Goal**: INSERT ON CONFLICT UPDATE support

**Tasks**:
1. Python `upsert()` API
   - Class method: `User.upsert(...)`
   - Instance method: `user.upsert(...)`
   - Batch: `User.upsert_many(...)`
2. Rust `UpsertBuilder`
   - Build `ON CONFLICT` SQL
   - Support multiple conflict targets
   - Support selective column updates
3. Parameter binding
   - PostgreSQL `$1, $2, ...` placeholders
   - Efficient batch upserts
4. Tests
   - Single/batch upsert
   - Various conflict strategies
   - Edge cases (NULL, composite keys)

**Deliverables**:
- `upsert()` in `python/data_bridge/postgres/table.py`
- `UpsertBuilder` in `crates/data-bridge-postgres/src/upsert.rs`
- Comprehensive upsert tests

**Complexity**: Low-Medium
**Dependencies**: None (independent feature)

---

### Phase 5: Auto-Migration Generation (3-4 weeks)

**Goal**: Automatic migration file generation from schema differences

**Tasks**:
1. Schema introspection
   - Extract ORM schema from `Table` classes
   - Query database schema (already implemented)
   - Compare the two
2. Rust `MigrationGenerator`
   - `SchemaDiff` struct
   - Table/column/index/FK comparison
   - SQL generation for UP/DOWN migrations
3. Python `migration.generate()` API
   - Collect all `Table` subclasses
   - Pass to Rust generator
   - Write migration file
4. Safety checks
   - Detect destructive changes (DROP COLUMN)
   - Warn about data loss risks
   - Require confirmation for risky migrations
5. Tests
   - Schema diff accuracy
   - SQL correctness (UP/DOWN)
   - Integration with existing migration system

**Deliverables**:
- `MigrationGenerator` in `crates/data-bridge-postgres/src/migration_generator.rs`
- `migration.generate()` in `python/data_bridge/postgres/migrations.py`
- Auto-migration tests

**Complexity**: High
**Dependencies**: Phase 1 (for FK support in schema)

---

### Summary Table

| Phase | Feature | Duration | Complexity | Dependencies |
|-------|---------|----------|------------|--------------|
| 1 | ForeignKey Storage | 2-3 weeks | Medium | None |
| 2 | JOIN Support | 3-4 weeks | High | Phase 1 |
| 3 | BackReference & Cascade | 2-3 weeks | Medium | Phase 1, 2 |
| 4 | Upsert | 1-2 weeks | Low-Medium | None |
| 5 | Auto-Migration | 3-4 weeks | High | Phase 1 |

**Total Estimated Time**: 11-16 weeks (3-4 months)

---

## Appendix A: MongoDB vs SQL Relationship Comparison

| Feature | MongoDB (Link/BackLink) | PostgreSQL (ForeignKey/BackReference) |
|---------|-------------------------|---------------------------------------|
| **Storage** | ObjectId string in document | Integer foreign key column |
| **Lazy Load** | Separate query by `_id` | Separate query by `id` |
| **Eager Load** | `$in` query with batching | JOIN query |
| **Constraints** | Application-level only | Database-level (optional) |
| **Cascade Delete** | Application-level | Database-level (ON DELETE CASCADE) |
| **Referential Integrity** | None (manual checking) | Enforced by database |
| **Performance** | Good (indexed `_id`) | Excellent (JOIN optimization) |

---

## Appendix B: Example Use Case

Complete example showing all features working together:

```python
from data_bridge.postgres import Table, Column, ForeignKey, BackReference, WriteRules, DeleteRules

# Define models
class User(Table):
    email: str = Column(unique=True)
    name: str
    # Reverse relationship
    posts: BackReference["Post"] = BackReference(foreign_key="author")

    class Settings:
        table_name = "users"

class Post(Table):
    title: str
    content: str
    # Forward relationship
    author: ForeignKey[User] = ForeignKey(on_delete="CASCADE")

    class Settings:
        table_name = "posts"

# Create schema
await migration.apply()

# Insert with cascade save
user = User(email="alice@example.com", name="Alice")
post = Post(title="Hello World", content="...", author=user)
await post.save(link_rule=WriteRules.WRITE)  # Saves user first

# Upsert
await User.upsert(
    {"email": "alice@example.com", "name": "Alice Updated"},
    conflict_target="email",
)

# Query with eager loading
posts = await Post.find().fetch_links("author").to_list()
for post in posts:
    print(f"{post.title} by {post.author.name}")  # No N+1 queries

# Delete with cascade
user = await User.get(1)
await user.delete(link_rule=DeleteRules.DELETE_LINKS)  # Deletes all posts too

# Auto-generate migration after adding new field
class User(Table):
    email: str = Column(unique=True)
    name: str
    status: str = Column(default="active")  # NEW FIELD
    posts: BackReference["Post"] = BackReference(foreign_key="author")

await migration.generate("add_user_status")
# Creates: migrations/20250129_150000_add_user_status.sql
await migration.apply()
```

---

## Appendix C: Performance Benchmarks (Target)

| Operation | Without Relationships | With Relationships | Target |
|-----------|----------------------|-------------------|--------|
| Insert 1000 posts (no FK) | 15ms | N/A | <20ms |
| Insert 1000 posts (with FK) | N/A | 18ms | <25ms |
| Find 1000 posts (lazy) | 6ms | 6ms | <10ms |
| Find 1000 posts (eager JOIN) | N/A | 12ms | <20ms |
| Upsert 1000 rows | N/A | 20ms | <30ms |
| Auto-generate migration | N/A | 50ms | <100ms |

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-12-29 | Claude Sonnet 4.5 | Initial design document |

