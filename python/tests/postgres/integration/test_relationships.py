"""
Integration tests for PostgreSQL foreign key relationships.

These tests verify:
- Foreign key schema introspection (get_foreign_keys)
- Querying by foreign key (find_by_foreign_key)
- ForeignKeyProxy functionality
"""

import pytest
from ouroboros.postgres import (
    execute,
    get_foreign_keys,
    find_by_foreign_key,
    Column,
    ForeignKeyProxy,
)
from ouroboros.test import expect


@pytest.mark.asyncio
async def test_get_foreign_keys_basic():
    """Test retrieving foreign key information from schema."""
    # Create users table
    await execute("""
        CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Create posts table with foreign key
    await execute("""
        CREATE TABLE posts (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            author_id INTEGER REFERENCES users(id) ON DELETE CASCADE
        )
    """)

    # Get foreign keys
    foreign_keys = await get_foreign_keys("posts")

    # Verify results
    expect(len(foreign_keys)).to_equal(1)
    fk = foreign_keys[0]
    expect("author_id" in fk["columns"]).to_be_true()
    expect(fk["referenced_table"]).to_equal("users")
    expect(fk["on_delete"]).to_equal("CASCADE")


@pytest.mark.asyncio
async def test_get_foreign_keys_multiple():
    """Test retrieving multiple foreign keys from a table."""
    # Create base tables
    await execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))")
    await execute("CREATE TABLE categories (id SERIAL PRIMARY KEY, title VARCHAR(255))")

    # Create posts table with multiple foreign keys
    await execute("""
        CREATE TABLE posts (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255),
            author_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
            category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL
        )
    """)

    # Get foreign keys
    foreign_keys = await get_foreign_keys("posts")

    # Verify results
    expect(len(foreign_keys)).to_equal(2)

    # Check that both foreign keys are present
    fk_tables = {fk["referenced_table"] for fk in foreign_keys}
    expect("users" in fk_tables).to_be_true()
    expect("categories" in fk_tables).to_be_true()

    # Check cascade rules
    for fk in foreign_keys:
        if fk["referenced_table"] == "users":
            expect(fk["on_delete"]).to_equal("CASCADE")
        elif fk["referenced_table"] == "categories":
            expect(fk["on_delete"]).to_equal("SET NULL")


@pytest.mark.asyncio
async def test_get_foreign_keys_no_foreign_keys():
    """Test get_foreign_keys on table without foreign keys."""
    await execute("""
        CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255)
        )
    """)

    foreign_keys = await get_foreign_keys("users")
    expect(len(foreign_keys)).to_equal(0)


@pytest.mark.asyncio
async def test_find_by_foreign_key():
    """Test querying related objects via foreign key."""
    # Create tables
    await execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))")
    await execute("""
        CREATE TABLE posts (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255),
            author_id INTEGER REFERENCES users(id)
        )
    """)

    # Insert data - use a simple insert and then query
    await execute(
        "INSERT INTO users (id, name) VALUES (1, $1)",
        ["Alice"]
    )
    user_id = 1

    await execute(
        "INSERT INTO posts (title, author_id) VALUES ($1, $2)",
        ["First Post", user_id]
    )

    # Query by foreign key
    author = await find_by_foreign_key("users", "id", user_id)

    # Verify result
    expect(author).not_to_be_none()
    expect(author["name"]).to_equal("Alice")
    expect(author["id"]).to_equal(user_id)


@pytest.mark.asyncio
async def test_find_by_foreign_key_not_found():
    """Test find_by_foreign_key when no row matches."""
    await execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))")

    # Query non-existent ID
    result = await find_by_foreign_key("users", "id", 99999)

    expect(result).to_be_none()


@pytest.mark.asyncio
async def test_column_foreign_key_parameter():
    """Test Column class accepts foreign_key parameter."""
    # Create column with foreign key
    col = Column(foreign_key="users.id", nullable=False)

    expect(col.foreign_key).to_equal("users.id")
    expect(col.nullable).to_equal(False)

    # Verify __repr__ includes foreign_key
    repr_str = repr(col)
    expect("foreign_key='users.id'" in repr_str).to_be_true()


@pytest.mark.asyncio
async def test_foreign_key_proxy_basic():
    """Test ForeignKeyProxy basic functionality."""
    # Create tables and data
    await execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))")
    await execute("""
        CREATE TABLE posts (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255),
            author_id INTEGER REFERENCES users(id)
        )
    """)

    await execute(
        "INSERT INTO users (id, name) VALUES (2, $1)",
        ["Bob"]
    )
    user_id = 2

    # Create ForeignKeyProxy
    proxy = ForeignKeyProxy("users", "id", user_id)

    # Test ref property (should work without fetching)
    expect(proxy.ref).to_equal(user_id)
    expect(proxy.id).to_equal(user_id)
    expect(proxy.column_value).to_equal(user_id)
    expect(proxy.is_fetched).to_be_false()

    # Test fetch
    author = await proxy.fetch()
    expect(author).not_to_be_none()
    expect(author["name"]).to_equal("Bob")
    expect(proxy.is_fetched).to_be_true()

    # Test cached fetch (should return same result without re-querying)
    author2 = await proxy.fetch()
    expect(author2).to_equal(author)


@pytest.mark.asyncio
async def test_foreign_key_proxy_not_found():
    """Test ForeignKeyProxy when referenced row doesn't exist."""
    await execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))")

    # Create proxy with non-existent ID
    proxy = ForeignKeyProxy("users", "id", 99999)

    # Fetch should return None
    result = await proxy.fetch()
    expect(result).to_be_none()
    expect(proxy.is_fetched).to_be_true()


@pytest.mark.asyncio
async def test_foreign_key_cascade_delete():
    """Test ON DELETE CASCADE behavior."""
    # Create tables with CASCADE
    await execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))")
    await execute("""
        CREATE TABLE posts (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255),
            author_id INTEGER REFERENCES users(id) ON DELETE CASCADE
        )
    """)

    # Insert data
    await execute(
        "INSERT INTO users (id, name) VALUES (3, $1)",
        ["Charlie"]
    )
    user_id = 3

    await execute(
        "INSERT INTO posts (title, author_id) VALUES ($1, $2)",
        ["Post 1", user_id]
    )
    await execute(
        "INSERT INTO posts (title, author_id) VALUES ($1, $2)",
        ["Post 2", user_id]
    )

    # Verify posts exist
    posts_before = await execute("SELECT * FROM posts WHERE author_id = $1", [user_id])
    expect(len(posts_before)).to_equal(2)

    # Delete user (should cascade to posts)
    await execute("DELETE FROM users WHERE id = $1", [user_id])

    # Verify posts were deleted
    posts_after = await execute("SELECT * FROM posts WHERE author_id = $1", [user_id])
    expect(len(posts_after)).to_equal(0)


@pytest.mark.asyncio
async def test_composite_foreign_key():
    """Test composite foreign keys (multiple columns)."""
    # Create tables with composite primary key
    await execute("""
        CREATE TABLE compound_keys (
            key1 INTEGER,
            key2 INTEGER,
            value VARCHAR(255),
            PRIMARY KEY (key1, key2)
        )
    """)

    await execute("""
        CREATE TABLE ref_table (
            id SERIAL PRIMARY KEY,
            ref_key1 INTEGER,
            ref_key2 INTEGER,
            FOREIGN KEY (ref_key1, ref_key2) REFERENCES compound_keys(key1, key2)
        )
    """)

    # Get foreign keys
    foreign_keys = await get_foreign_keys("ref_table")

    # Verify composite foreign key
    expect(len(foreign_keys)).to_equal(1)
    fk = foreign_keys[0]
    expect(len(fk["columns"])).to_equal(2)
    expect("ref_key1" in fk["columns"]).to_be_true()
    expect("ref_key2" in fk["columns"]).to_be_true()
    expect(fk["referenced_table"]).to_equal("compound_keys")
    expect(len(fk["referenced_columns"])).to_equal(2)
