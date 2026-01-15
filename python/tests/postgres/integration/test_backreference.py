"""
Integration tests for PostgreSQL BackReference descriptor.

These tests verify:
- BackReference.fetch_all() - Fetch all related rows
- BackReference.fetch_one() - Fetch first related row
- BackReference.count() - Count related rows
- BackReference behavior with no results
- BackReference.ref_value property
- Class vs instance access patterns
"""

import pytest
from ouroboros.postgres import (
    execute,
    insert_one,
    BackReference,
    BackReferenceQuery,
)
from ouroboros.qc import expect


@pytest.mark.asyncio
async def test_backreference_fetch_all():
    """Test fetching all related rows via BackReference."""
    # Create users table
    await execute("""
        CREATE TABLE br_users_1 (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Create posts table with foreign key to users
    await execute("""
        CREATE TABLE br_posts_1 (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            user_id INTEGER REFERENCES br_users_1(id) ON DELETE CASCADE
        )
    """)

    # Insert test user
    await execute(
        "INSERT INTO br_users_1 (id, name) VALUES (1, $1)",
        ["Alice"]
    )
    user_id = 1

    # Insert multiple posts for the user
    await execute(
        "INSERT INTO br_posts_1 (title, user_id) VALUES ($1, $2)",
        ["First Post", user_id]
    )
    await execute(
        "INSERT INTO br_posts_1 (title, user_id) VALUES ($1, $2)",
        ["Second Post", user_id]
    )
    await execute(
        "INSERT INTO br_posts_1 (title, user_id) VALUES ($1, $2)",
        ["Third Post", user_id]
    )

    # Create BackReferenceQuery manually (simulating instance access)
    back_ref_query = BackReferenceQuery("br_posts_1", "user_id", "id", user_id)

    # Fetch all posts for the user
    posts = await back_ref_query.fetch_all()

    # Verify results
    expect(len(posts)).to_equal(3)
    titles = {post["title"] for post in posts}
    expect("First Post" in titles).to_be_true()
    expect("Second Post" in titles).to_be_true()
    expect("Third Post" in titles).to_be_true()

    # Verify all posts have correct user_id
    for post in posts:
        expect(post["user_id"]).to_equal(user_id)


@pytest.mark.asyncio
async def test_backreference_fetch_one():
    """Test fetching first related row via BackReference."""
    # Create users table
    await execute("""
        CREATE TABLE br_users_2 (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Create posts table with foreign key to users
    await execute("""
        CREATE TABLE br_posts_2 (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            user_id INTEGER REFERENCES br_users_2(id),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)

    # Insert test user
    await execute(
        "INSERT INTO br_users_2 (id, name) VALUES (2, $1)",
        ["Bob"]
    )
    user_id = 2

    # Insert multiple posts for the user
    await execute(
        "INSERT INTO br_posts_2 (title, user_id) VALUES ($1, $2)",
        ["First Post", user_id]
    )
    await execute(
        "INSERT INTO br_posts_2 (title, user_id) VALUES ($1, $2)",
        ["Second Post", user_id]
    )

    # Create BackReferenceQuery
    back_ref_query = BackReferenceQuery("br_posts_2", "user_id", "id", user_id)

    # Fetch one post for the user
    first_post = await back_ref_query.fetch_one()

    # Verify result
    expect(first_post is not None).to_be_true()
    expect(first_post["user_id"]).to_equal(user_id)
    expect(first_post["title"] in ["First Post", "Second Post"]).to_be_true()


@pytest.mark.asyncio
async def test_backreference_count():
    """Test counting related rows via BackReference."""
    # Create users table
    await execute("""
        CREATE TABLE br_users_3 (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Create posts table with foreign key to users
    await execute("""
        CREATE TABLE br_posts_3 (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            user_id INTEGER REFERENCES br_users_3(id)
        )
    """)

    # Insert test users
    await execute(
        "INSERT INTO br_users_3 (id, name) VALUES (3, $1)",
        ["Charlie"]
    )
    user_id = 3

    # Insert posts for the user
    await execute(
        "INSERT INTO br_posts_3 (title, user_id) VALUES ($1, $2)",
        ["Post 1", user_id]
    )
    await execute(
        "INSERT INTO br_posts_3 (title, user_id) VALUES ($1, $2)",
        ["Post 2", user_id]
    )
    await execute(
        "INSERT INTO br_posts_3 (title, user_id) VALUES ($1, $2)",
        ["Post 3", user_id]
    )
    await execute(
        "INSERT INTO br_posts_3 (title, user_id) VALUES ($1, $2)",
        ["Post 4", user_id]
    )

    # Create BackReferenceQuery
    back_ref_query = BackReferenceQuery("br_posts_3", "user_id", "id", user_id)

    # Count posts for the user
    post_count = await back_ref_query.count()

    # Verify count
    expect(post_count).to_equal(4)


@pytest.mark.asyncio
async def test_backreference_no_results():
    """Test BackReference with no related rows returns empty list."""
    # Create users table
    await execute("""
        CREATE TABLE br_users_4 (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Create posts table with foreign key to users
    await execute("""
        CREATE TABLE br_posts_4 (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            user_id INTEGER REFERENCES br_users_4(id)
        )
    """)

    # Insert test user with no posts
    await execute(
        "INSERT INTO br_users_4 (id, name) VALUES (4, $1)",
        ["David"]
    )
    user_id = 4

    # Create BackReferenceQuery
    back_ref_query = BackReferenceQuery("br_posts_4", "user_id", "id", user_id)

    # Fetch all posts (should be empty)
    posts = await back_ref_query.fetch_all()
    expect(len(posts)).to_equal(0)
    expect(posts).to_equal([])

    # Fetch one post (should be None)
    first_post = await back_ref_query.fetch_one()
    expect(first_post).to_be_none()

    # Count posts (should be 0)
    post_count = await back_ref_query.count()
    expect(post_count).to_equal(0)


@pytest.mark.asyncio
async def test_backreference_ref_value():
    """Test BackReferenceQuery.ref_value property returns correct value."""
    # Create tables
    await execute("""
        CREATE TABLE br_users_5 (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    await execute("""
        CREATE TABLE br_posts_5 (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            user_id INTEGER REFERENCES br_users_5(id)
        )
    """)

    # Insert test user
    await execute(
        "INSERT INTO br_users_5 (id, name) VALUES (5, $1)",
        ["Eve"]
    )
    user_id = 5

    # Create BackReferenceQuery
    back_ref_query = BackReferenceQuery("br_posts_5", "user_id", "id", user_id)

    # Verify ref_value property
    expect(back_ref_query.ref_value).to_equal(user_id)
    expect(back_ref_query.ref_value).to_equal(5)


@pytest.mark.asyncio
async def test_backreference_class_access():
    """Test accessing BackReference descriptor on class returns descriptor itself."""
    # Create a simple mock class to test descriptor behavior
    class MockUser:
        posts = BackReference("posts", "user_id", "id")

    # Accessing on class should return the descriptor
    descriptor = MockUser.posts

    # Verify it's the BackReference descriptor
    expect(isinstance(descriptor, BackReference)).to_be_true()
    expect(descriptor.source_table).to_equal("posts")
    expect(descriptor.source_column).to_equal("user_id")
    expect(descriptor.target_column).to_equal("id")


@pytest.mark.asyncio
async def test_backreference_instance_access():
    """Test accessing BackReference descriptor on instance returns BackReferenceQuery."""
    # Create tables
    await execute("""
        CREATE TABLE br_users_6 (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    await execute("""
        CREATE TABLE br_posts_6 (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            user_id INTEGER REFERENCES br_users_6(id)
        )
    """)

    # Create a simple mock instance
    class MockUser:
        posts = BackReference("posts", "user_id", "id")

        def __init__(self, user_id):
            self._data = {"id": user_id}

    # Create instance
    user = MockUser(user_id=6)

    # Accessing on instance should return BackReferenceQuery
    query = user.posts

    # Verify it's a BackReferenceQuery
    expect(isinstance(query, BackReferenceQuery)).to_be_true()
    expect(query.source_table).to_equal("posts")
    expect(query.source_column).to_equal("user_id")
    expect(query.target_column).to_equal("id")
    expect(query.ref_value).to_equal(6)


@pytest.mark.asyncio
async def test_backreference_null_ref_value():
    """Test BackReference behavior when ref_value is None."""
    # Create BackReferenceQuery with None ref_value
    back_ref_query = BackReferenceQuery("posts", "user_id", "id", None)

    # fetch_all should return empty list
    posts = await back_ref_query.fetch_all()
    expect(posts).to_equal([])

    # fetch_one should return None
    first_post = await back_ref_query.fetch_one()
    expect(first_post).to_be_none()

    # count should return 0
    post_count = await back_ref_query.count()
    expect(post_count).to_equal(0)


@pytest.mark.asyncio
async def test_backreference_multiple_users():
    """Test BackReference correctly filters by different user IDs."""
    # Create users table
    await execute("""
        CREATE TABLE br_users_7 (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Create posts table with foreign key to users
    await execute("""
        CREATE TABLE br_posts_7 (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            user_id INTEGER REFERENCES br_users_7(id)
        )
    """)

    # Insert test users
    await execute("INSERT INTO br_users_7 (id, name) VALUES (7, $1)", ["User1"])
    await execute("INSERT INTO br_users_7 (id, name) VALUES (8, $1)", ["User2"])

    # Insert posts for user 7
    await execute(
        "INSERT INTO br_posts_7 (title, user_id) VALUES ($1, $2)",
        ["User1 Post 1", 7]
    )
    await execute(
        "INSERT INTO br_posts_7 (title, user_id) VALUES ($1, $2)",
        ["User1 Post 2", 7]
    )

    # Insert posts for user 8
    await execute(
        "INSERT INTO br_posts_7 (title, user_id) VALUES ($1, $2)",
        ["User2 Post 1", 8]
    )
    await execute(
        "INSERT INTO br_posts_7 (title, user_id) VALUES ($1, $2)",
        ["User2 Post 2", 8]
    )
    await execute(
        "INSERT INTO br_posts_7 (title, user_id) VALUES ($1, $2)",
        ["User2 Post 3", 8]
    )

    # Create BackReferenceQuery for user 7
    user7_query = BackReferenceQuery("br_posts_7", "user_id", "id", 7)
    user7_posts = await user7_query.fetch_all()
    expect(len(user7_posts)).to_equal(2)

    # Create BackReferenceQuery for user 8
    user8_query = BackReferenceQuery("br_posts_7", "user_id", "id", 8)
    user8_posts = await user8_query.fetch_all()
    expect(len(user8_posts)).to_equal(3)

    # Verify correct posts for each user
    for post in user7_posts:
        expect(post["user_id"]).to_equal(7)
        expect(post["title"].startswith("User1")).to_be_true()

    for post in user8_posts:
        expect(post["user_id"]).to_equal(8)
        expect(post["title"].startswith("User2")).to_be_true()


@pytest.mark.asyncio
async def test_backreference_repr():
    """Test __repr__ method of BackReference and BackReferenceQuery."""
    # Test BackReference __repr__
    back_ref = BackReference("posts", "user_id", "id")
    repr_str = repr(back_ref)
    expect("posts" in repr_str).to_be_true()
    expect("user_id" in repr_str).to_be_true()
    expect("id" in repr_str).to_be_true()

    # Test BackReferenceQuery __repr__
    query = BackReferenceQuery("posts", "user_id", "id", 123)
    query_repr = repr(query)
    expect("posts" in query_repr).to_be_true()
    expect("user_id" in query_repr).to_be_true()
    expect("123" in query_repr).to_be_true()


@pytest.mark.asyncio
async def test_backreference_custom_target_column():
    """Test BackReference with custom target_column (not 'id')."""
    # Create users table with uuid as primary key
    await execute("""
        CREATE TABLE br_users_8 (
            user_uuid VARCHAR(36) PRIMARY KEY,
            name VARCHAR(255) NOT NULL
        )
    """)

    # Create posts table with foreign key to users.user_uuid
    await execute("""
        CREATE TABLE br_posts_8 (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            author_uuid VARCHAR(36) REFERENCES br_users_8(user_uuid)
        )
    """)

    # Insert test user
    user_uuid = "550e8400-e29b-41d4-a716-446655440000"
    await execute(
        "INSERT INTO br_users_8 (user_uuid, name) VALUES ($1, $2)",
        [user_uuid, "Frank"]
    )

    # Insert posts for the user
    await execute(
        "INSERT INTO br_posts_8 (title, author_uuid) VALUES ($1, $2)",
        ["Post 1", user_uuid]
    )
    await execute(
        "INSERT INTO br_posts_8 (title, author_uuid) VALUES ($1, $2)",
        ["Post 2", user_uuid]
    )

    # Create BackReferenceQuery with custom target_column
    back_ref_query = BackReferenceQuery("br_posts_8", "author_uuid", "user_uuid", user_uuid)

    # Fetch posts
    posts = await back_ref_query.fetch_all()

    # Verify results
    expect(len(posts)).to_equal(2)
    for post in posts:
        expect(post["author_uuid"]).to_equal(user_uuid)

    # Count posts
    post_count = await back_ref_query.count()
    expect(post_count).to_equal(2)
