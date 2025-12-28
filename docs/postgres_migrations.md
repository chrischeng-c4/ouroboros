# PostgreSQL Migrations

The data-bridge PostgreSQL migration system provides a production-ready way to manage database schema evolution, similar to Alembic or Rails migrations but implemented in Rust for maximum performance and reliability.

## Features

- **Version Tracking**: Timestamp-based migration versions prevent conflicts
- **Transactional**: All migrations run in transactions (all-or-nothing)
- **Checksum Validation**: Detect modified migrations after application
- **Up/Down Migrations**: Support for both forward and rollback migrations
- **Directory-Based**: Simple file-based migration management
- **Parallel Safe**: Multiple developers can create migrations independently

## Quick Start

### 1. Initialize Migration System

Create the `_migrations` tracking table:

```python
import asyncio
from data_bridge import postgres

async def init():
    await postgres.init("postgresql://localhost/mydb")
    await postgres.migration_init()

asyncio.run(init())
```

### 2. Create a Migration

```python
# Creates a new migration file with timestamp
filepath = postgres.migration_create(
    "create users table",
    migrations_dir="migrations"
)
print(f"Created: {filepath}")
```

This creates a file like `migrations/20250128_120000_create_users_table.sql`:

```sql
-- Migration: 20250128_120000_create_users_table
-- Description: create users table

-- UP
CREATE TABLE example (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- DOWN
DROP TABLE IF EXISTS example CASCADE;
```

### 3. Edit the Migration

Edit the generated file to add your actual SQL:

```sql
-- Migration: 20250128_120000_create_users_table
-- Description: Create users table with basic columns

-- UP
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);

-- DOWN
DROP TABLE IF EXISTS users CASCADE;
```

### 4. Apply Migrations

```python
# Check what will be applied
status = await postgres.migration_status("migrations")
print(f"Pending: {status['pending']}")

# Apply all pending migrations
applied = await postgres.migration_apply("migrations")
print(f"Applied: {applied}")
```

### 5. Rollback if Needed

```python
# Rollback last migration
reverted = await postgres.migration_rollback("migrations", steps=1)
print(f"Reverted: {reverted}")
```

## Migration File Format

### File Naming Convention

Migrations must follow this naming pattern:

```
YYYYMMDD_HHMMSS_description.sql
```

Examples:
- `20250128_120000_create_users_table.sql`
- `20250128_130000_add_user_status.sql`
- `20250129_093000_create_posts_table.sql`

The timestamp ensures:
1. Migrations are applied in order
2. Multiple developers can create migrations without conflicts
3. Clear chronological history

### File Structure

Each migration file has three sections:

```sql
-- Migration: [auto-generated from filename]
-- Description: [human-readable description]

-- UP
[SQL statements to apply migration]

-- DOWN
[SQL statements to revert migration]
```

**Important:**
- Both `-- UP` and `-- DOWN` sections are required
- SQL in each section can be multiple statements
- Always test your DOWN migration!

## API Reference

### `migration_init()`

Initialize the migration tracking system.

```python
await postgres.migration_init()
```

Creates the `_migrations` table with schema:
```sql
CREATE TABLE _migrations (
    version VARCHAR(255) PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    checksum VARCHAR(64) NOT NULL
);
```

### `migration_status(migrations_dir)`

Get current migration status.

```python
status = await postgres.migration_status("migrations")
print(status)
# {
#     'applied': ['20250128_120000', '20250128_130000'],
#     'pending': ['20250128_140000']
# }
```

**Parameters:**
- `migrations_dir` (str): Path to directory containing migration files

**Returns:**
- Dictionary with `applied` and `pending` lists

### `migration_apply(migrations_dir)`

Apply all pending migrations.

```python
applied = await postgres.migration_apply("migrations")
print(applied)  # ['20250128_140000', '20250128_150000']
```

**Parameters:**
- `migrations_dir` (str): Path to directory containing migration files

**Returns:**
- List of applied migration versions

**Behavior:**
- Migrations are applied in order by version
- Each migration runs in a transaction
- If any migration fails, the transaction is rolled back
- Applied migrations are recorded in `_migrations` table

### `migration_rollback(migrations_dir, steps=1)`

Rollback the last N migrations.

```python
# Rollback last migration
reverted = await postgres.migration_rollback("migrations", steps=1)

# Rollback last 3 migrations
reverted = await postgres.migration_rollback("migrations", steps=3)
```

**Parameters:**
- `migrations_dir` (str): Path to directory containing migration files
- `steps` (int): Number of migrations to rollback (default: 1)

**Returns:**
- List of reverted migration versions

**Behavior:**
- Migrations are reverted in reverse order
- Each rollback runs in a transaction
- Migration records are removed from `_migrations` table

### `migration_create(description, migrations_dir="migrations")`

Create a new migration file.

```python
filepath = postgres.migration_create(
    "add user profile fields",
    migrations_dir="migrations"
)
print(filepath)  # migrations/20250128_153000_add_user_profile_fields.sql
```

**Parameters:**
- `description` (str): Human-readable description
- `migrations_dir` (str): Directory to create file in (default: "migrations")

**Returns:**
- Path to created migration file

**Behavior:**
- Creates migrations directory if it doesn't exist
- Generates timestamp-based version
- Creates file with template UP/DOWN sections

## Best Practices

### 1. Always Write Reversible Migrations

Every migration should be reversible. Test your DOWN migration:

```sql
-- UP
ALTER TABLE users ADD COLUMN last_login TIMESTAMPTZ;

-- DOWN
ALTER TABLE users DROP COLUMN last_login;
```

### 2. Use Transactions Wisely

PostgreSQL DDL is transactional, but some operations can't be rolled back:
- Index creation (`CREATE INDEX CONCURRENTLY`)
- Certain ALTER TYPE operations

For these cases, consider:
```sql
-- UP
-- Note: This migration cannot be fully rolled back
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);

-- DOWN
-- Best effort rollback
DROP INDEX IF EXISTS idx_users_email;
```

### 3. Break Large Migrations into Steps

Instead of one massive migration:

```sql
-- BAD: One huge migration
-- UP
CREATE TABLE a ...;
CREATE TABLE b ...;
CREATE TABLE c ...;
-- 50 more tables...
```

Split into logical chunks:

```sql
-- 001_create_user_tables.sql
-- 002_create_content_tables.sql
-- 003_create_analytics_tables.sql
```

### 4. Handle Data Migration Carefully

When migrating data, consider:

```sql
-- UP
-- 1. Add new column (nullable first)
ALTER TABLE users ADD COLUMN full_name VARCHAR(500);

-- 2. Migrate data
UPDATE users SET full_name = CONCAT(first_name, ' ', last_name);

-- 3. Make it NOT NULL
ALTER TABLE users ALTER COLUMN full_name SET NOT NULL;

-- DOWN
ALTER TABLE users DROP COLUMN full_name;
```

### 5. Add Indexes Carefully

For large tables, use `CONCURRENTLY`:

```sql
-- UP
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);

-- DOWN
DROP INDEX CONCURRENTLY IF EXISTS idx_users_email;
```

### 6. Use Constraints for Data Integrity

```sql
-- UP
ALTER TABLE users
    ADD CONSTRAINT check_email_format
    CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}$');

-- DOWN
ALTER TABLE users DROP CONSTRAINT check_email_format;
```

### 7. Document Complex Migrations

```sql
-- Migration: 20250128_120000_restructure_user_roles
-- Description: Migrate from role_id to role_name for better flexibility
--
-- IMPORTANT: This migration will:
-- 1. Create new role_name column
-- 2. Migrate data from users_roles table
-- 3. Drop old role_id column
--
-- Rollback will restore role_id from role_name mapping

-- UP
-- ... migration SQL
```

## Common Patterns

### Adding a Column

```sql
-- UP
ALTER TABLE users
    ADD COLUMN phone VARCHAR(20),
    ADD COLUMN verified BOOLEAN DEFAULT false;

-- DOWN
ALTER TABLE users
    DROP COLUMN phone,
    DROP COLUMN verified;
```

### Renaming a Column

```sql
-- UP
ALTER TABLE users RENAME COLUMN username TO email;

-- DOWN
ALTER TABLE users RENAME COLUMN email TO username;
```

### Creating a Table with Relationships

```sql
-- UP
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL,
    content TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_posts_user_id ON posts(user_id);

-- DOWN
DROP TABLE IF EXISTS posts CASCADE;
```

### Adding Triggers

```sql
-- UP
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_users_timestamp
BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- DOWN
DROP TRIGGER IF EXISTS update_users_timestamp ON users;
DROP FUNCTION IF EXISTS update_updated_at();
```

## Security

### Checksum Validation

Every migration has a SHA256 checksum calculated from its content. If you try to modify a migration after it's been applied, you'll get an error:

```
Error: Checksum mismatch for migration 20250128_120000.
The migration file has been modified after being applied.
```

**Rule:** Never modify a migration after it's been applied to production. Create a new migration instead.

### SQL Injection Protection

The migration system uses parameterized queries for tracking operations. However, migration SQL itself is executed directly, so:

1. Never generate migration SQL from user input
2. Review all migrations before applying to production
3. Use code review for all migrations

## Troubleshooting

### Migration Failed Midway

If a migration fails:
1. The transaction is rolled back (no partial changes)
2. The migration is not recorded as applied
3. Fix the migration SQL
4. Run `migration_apply()` again

### Applied Migration Not in Files

If a migration is recorded but the file is missing:
```python
# Error: Migration 20250128_120000 is applied but not found in migration files
```

This happens if you deleted a migration file after applying it. Solutions:
1. Restore the migration file from version control
2. Manually remove the record from `_migrations` table (dangerous!)

### Conflicting Migrations

If two developers create migrations at the same time:
```
migrations/
├── 20250128_120000_alice_feature.sql   (Alice)
└── 20250128_120000_bob_feature.sql     (Bob)
```

Resolution:
1. Rename one migration to a later timestamp
2. Or merge them if they're related

### Checksum Mismatch

If you need to modify an applied migration (not recommended):
1. Create a new migration with the changes
2. Or manually update the checksum in `_migrations` table (dangerous!)

## Production Workflow

### Development

```bash
# 1. Create migration
python -c "from data_bridge import postgres; print(postgres.migration_create('add feature'))"

# 2. Edit the migration file

# 3. Apply locally
python -c "import asyncio; from data_bridge import postgres; asyncio.run(postgres.migration_apply('migrations'))"

# 4. Test rollback
python -c "import asyncio; from data_bridge import postgres; asyncio.run(postgres.migration_rollback('migrations'))"

# 5. Re-apply
python -c "import asyncio; from data_bridge import postgres; asyncio.run(postgres.migration_apply('migrations'))"

# 6. Commit migration file to version control
```

### Staging/Production

```bash
# 1. Pull latest code (includes new migrations)
git pull

# 2. Check pending migrations
python -c "import asyncio; from data_bridge import postgres; asyncio.run(postgres.migration_status('migrations'))"

# 3. Backup database (important!)
pg_dump mydb > backup.sql

# 4. Apply migrations
python -c "import asyncio; from data_bridge import postgres; asyncio.run(postgres.migration_apply('migrations'))"

# 5. Verify application works
```

## Advanced Usage

### Custom Migration Table Name

```python
from data_bridge.postgres import MigrationRunner

runner = MigrationRunner(connection, migrations_table="my_custom_migrations")
await runner.init()
```

### Programmatic Migration Creation

```python
from data_bridge.postgres import Migration

migration = Migration.new(
    version="20250128_120000",
    name="create users",
    up="CREATE TABLE users (id SERIAL PRIMARY KEY);",
    down="DROP TABLE users;"
)

runner = MigrationRunner(connection)
await runner.apply(migration)
```

### Migration in Application Startup

```python
async def startup():
    await postgres.init("postgresql://localhost/mydb")
    await postgres.migration_init()

    # Auto-apply migrations on startup (development only!)
    if os.getenv("AUTO_MIGRATE") == "true":
        await postgres.migration_apply("migrations")
```

## Comparison with Other Tools

### vs Alembic

| Feature | data-bridge | Alembic |
|---------|-------------|---------|
| Language | Rust (Python API) | Python |
| Performance | 10-100x faster | Standard |
| Transactions | Built-in | Manual |
| Checksum | SHA256 | MD5 |
| Auto-generate | No (by design) | Yes |
| Dependencies | None (compiled) | Many Python packages |

### vs Flyway

| Feature | data-bridge | Flyway |
|---------|-------------|--------|
| Language | Rust | Java |
| File format | Simple SQL | SQL + Java |
| Checksum | SHA256 | CRC32 |
| Versioning | Timestamp | Sequential or timestamp |
| Rollback | Built-in | Commercial only |

## Examples

See the `examples/postgres_migrations/` directory for complete examples:
- `20250128_120000_create_users_table.sql` - Basic table creation
- `20250128_130000_add_users_status_column.sql` - Adding columns
- `20250128_140000_create_posts_table.sql` - Foreign keys and triggers

Run the example:
```bash
python examples/postgres_migrations_example.py
```

## License

MIT
