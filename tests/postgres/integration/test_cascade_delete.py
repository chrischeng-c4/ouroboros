"""Integration tests for cascade-aware delete operations.

This module tests cascade delete operations that handle foreign key relationships
and cascade rules via the PostgreSQL database.
"""

import pytest
from data_bridge.postgres import execute


@pytest.fixture
async def setup_cascade_tables():
    """Set up test tables with various cascade rules."""
    # Create parent table
    await execute("""
        CREATE TABLE IF NOT EXISTS cascade_users (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL
        );
    """)

    # Create child table with CASCADE on delete
    await execute("""
        CREATE TABLE IF NOT EXISTS cascade_posts (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            content TEXT,
            FOREIGN KEY (user_id) REFERENCES cascade_users(id) ON DELETE CASCADE
        );
    """)

    # Create grandchild table with RESTRICT on delete
    await execute("""
        CREATE TABLE IF NOT EXISTS cascade_comments (
            id SERIAL PRIMARY KEY,
            post_id INTEGER NOT NULL,
            user_id INTEGER,
            text TEXT NOT NULL,
            FOREIGN KEY (post_id) REFERENCES cascade_posts(id) ON DELETE RESTRICT,
            FOREIGN KEY (user_id) REFERENCES cascade_users(id) ON DELETE SET NULL
        );
    """)

    yield
    # Cleanup is handled by conftest.py cleanup_tables fixture


@pytest.mark.integration
@pytest.mark.asyncio
async def test_delete_with_cascade_basic(setup_cascade_tables):
    """Test basic cascade delete functionality."""
    # Insert test data
    await execute("""
        INSERT INTO cascade_users (name, email)
        VALUES ('Alice', 'alice@example.com')
    """)

    user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Alice'")
    user_id = user_result[0]["id"]

    await execute(f"""
        INSERT INTO cascade_posts (user_id, title, content)
        VALUES ({user_id}, 'My First Post', 'Hello world!')
    """)

    post_result = await execute("SELECT id FROM cascade_posts WHERE user_id = $1", [user_id])
    post_id = post_result[0]["id"]

    # Verify data exists
    found_user = await execute("SELECT * FROM cascade_users WHERE id = $1", [user_id])
    assert len(found_user) == 1

    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    assert len(found_post) == 1

    # Delete user with cascade (database handles cascade)
    await execute("DELETE FROM cascade_users WHERE id = $1", [user_id])

    # Verify both are deleted (CASCADE handled by database)
    found_user = await execute("SELECT * FROM cascade_users WHERE id = $1", [user_id])
    assert len(found_user) == 0

    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    assert len(found_post) == 0


@pytest.mark.integration
@pytest.mark.asyncio
async def test_delete_with_cascade_restrict_violation(setup_cascade_tables):
    """Test that RESTRICT constraint prevents deletion."""
    # Insert test data
    await execute("""
        INSERT INTO cascade_users (name, email)
        VALUES ('Bob', 'bob@example.com')
    """)

    user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Bob'")
    user_id = user_result[0]["id"]

    await execute(f"""
        INSERT INTO cascade_posts (user_id, title, content)
        VALUES ({user_id}, 'Bob''s Post', 'Some content')
    """)

    post_result = await execute("SELECT id FROM cascade_posts WHERE user_id = $1", [user_id])
    post_id = post_result[0]["id"]

    # Add a comment (has RESTRICT on post_id)
    await execute(f"""
        INSERT INTO cascade_comments (post_id, user_id, text)
        VALUES ({post_id}, {user_id}, 'Great post!')
    """)

    # Try to delete post - should fail due to RESTRICT
    with pytest.raises(Exception) as exc_info:
        await execute("DELETE FROM cascade_posts WHERE id = $1", [post_id])

    assert "violate" in str(exc_info.value).lower() or "restrict" in str(exc_info.value).lower()

    # Verify post still exists
    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    assert len(found_post) == 1


@pytest.mark.integration
@pytest.mark.asyncio
async def test_delete_with_cascade_set_null(setup_cascade_tables):
    """Test SET NULL cascade rule."""
    # Insert test data
    await execute("""
        INSERT INTO cascade_users (name, email)
        VALUES ('Charlie', 'charlie@example.com'), ('Dave', 'dave@example.com')
    """)

    user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Charlie'")
    user_id = user_result[0]["id"]

    other_user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Dave'")
    other_user_id = other_user_result[0]["id"]

    await execute(f"""
        INSERT INTO cascade_posts (user_id, title, content)
        VALUES ({other_user_id}, 'Post by Dave', 'Content')
    """)

    post_result = await execute("SELECT id FROM cascade_posts WHERE user_id = $1", [other_user_id])
    post_id = post_result[0]["id"]

    # Add comment by Charlie on Dave's post
    await execute(f"""
        INSERT INTO cascade_comments (post_id, user_id, text)
        VALUES ({post_id}, {user_id}, 'Nice post!')
    """)

    comment_result = await execute("SELECT id FROM cascade_comments WHERE user_id = $1", [user_id])
    comment_id = comment_result[0]["id"]

    # Delete Charlie (user_id in comments has SET NULL)
    await execute("DELETE FROM cascade_users WHERE id = $1", [user_id])

    # Verify comment still exists but user_id is NULL
    found_comment = await execute("SELECT * FROM cascade_comments WHERE id = $1", [comment_id])
    assert len(found_comment) == 1
    assert found_comment[0]["user_id"] is None
    assert found_comment[0]["text"] == "Nice post!"


@pytest.mark.integration
@pytest.mark.asyncio
async def test_delete_checked_allows_cascade(setup_cascade_tables):
    """Test cascade delete allows CASCADE rule."""
    # Insert test data
    await execute("""
        INSERT INTO cascade_users (name, email)
        VALUES ('Eve', 'eve@example.com')
    """)

    user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Eve'")
    user_id = user_result[0]["id"]

    await execute(f"""
        INSERT INTO cascade_posts (user_id, title, content)
        VALUES ({user_id}, 'Eve''s Post', 'Content')
    """)

    post_result = await execute("SELECT id FROM cascade_posts WHERE user_id = $1", [user_id])
    post_id = post_result[0]["id"]

    # Delete user with CASCADE
    # Should succeed because posts have CASCADE on delete
    await execute("DELETE FROM cascade_users WHERE id = $1", [user_id])

    # Both should be deleted (CASCADE handled by database)
    found_user = await execute("SELECT * FROM cascade_users WHERE id = $1", [user_id])
    assert len(found_user) == 0

    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    assert len(found_post) == 0


@pytest.mark.integration
@pytest.mark.asyncio
async def test_delete_checked_blocks_restrict(setup_cascade_tables):
    """Test that RESTRICT blocks deletion."""
    # Insert test data
    await execute("""
        INSERT INTO cascade_users (name, email)
        VALUES ('Frank', 'frank@example.com')
    """)

    user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Frank'")
    user_id = user_result[0]["id"]

    await execute(f"""
        INSERT INTO cascade_posts (user_id, title, content)
        VALUES ({user_id}, 'Frank''s Post', 'Content')
    """)

    post_result = await execute("SELECT id FROM cascade_posts WHERE user_id = $1", [user_id])
    post_id = post_result[0]["id"]

    await execute(f"""
        INSERT INTO cascade_comments (post_id, user_id, text)
        VALUES ({post_id}, {user_id}, 'Comment')
    """)

    # Try to delete post - should fail due to RESTRICT from comments
    with pytest.raises(Exception) as exc_info:
        await execute("DELETE FROM cascade_posts WHERE id = $1", [post_id])

    assert "violate" in str(exc_info.value).lower() or "restrict" in str(exc_info.value).lower()

    # Verify post still exists
    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    assert len(found_post) == 1


@pytest.mark.integration
@pytest.mark.asyncio
async def test_delete_with_cascade_no_children(setup_cascade_tables):
    """Test cascade delete when there are no child records."""
    # Insert user without posts
    await execute("""
        INSERT INTO cascade_users (name, email)
        VALUES ('Grace', 'grace@example.com')
    """)

    user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Grace'")
    user_id = user_result[0]["id"]

    # Delete should work fine
    await execute("DELETE FROM cascade_users WHERE id = $1", [user_id])

    # Verify user is deleted
    found_user = await execute("SELECT * FROM cascade_users WHERE id = $1", [user_id])
    assert len(found_user) == 0


@pytest.mark.integration
@pytest.mark.asyncio
async def test_delete_with_cascade_multiple_children(setup_cascade_tables):
    """Test cascade delete with multiple child records."""
    # Insert user with multiple posts
    await execute("""
        INSERT INTO cascade_users (name, email)
        VALUES ('Henry', 'henry@example.com')
    """)

    user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Henry'")
    user_id = user_result[0]["id"]

    # Create 3 posts
    post_ids = []
    for i in range(3):
        await execute(f"""
            INSERT INTO cascade_posts (user_id, title, content)
            VALUES ({user_id}, 'Post {i+1}', 'Content {i+1}')
        """)

    post_results = await execute("SELECT id FROM cascade_posts WHERE user_id = $1", [user_id])
    post_ids = [row["id"] for row in post_results]

    # Delete user with cascade
    await execute("DELETE FROM cascade_users WHERE id = $1", [user_id])

    # Verify all are deleted
    found_user = await execute("SELECT * FROM cascade_users WHERE id = $1", [user_id])
    assert len(found_user) == 0

    for post_id in post_ids:
        found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
        assert len(found_post) == 0


@pytest.mark.integration
@pytest.mark.asyncio
async def test_delete_with_cascade_transaction_rollback(setup_cascade_tables):
    """Test that failed cascade delete prevents deletion (RESTRICT constraint)."""
    # Insert test data
    await execute("""
        INSERT INTO cascade_users (name, email)
        VALUES ('Ivy', 'ivy@example.com')
    """)

    user_result = await execute("SELECT id FROM cascade_users WHERE name = 'Ivy'")
    user_id = user_result[0]["id"]

    await execute(f"""
        INSERT INTO cascade_posts (user_id, title, content)
        VALUES ({user_id}, 'Ivy''s Post', 'Content')
    """)

    post_result = await execute("SELECT id FROM cascade_posts WHERE user_id = $1", [user_id])
    post_id = post_result[0]["id"]

    await execute(f"""
        INSERT INTO cascade_comments (post_id, user_id, text)
        VALUES ({post_id}, {user_id}, 'Comment')
    """)

    comment_result = await execute("SELECT id FROM cascade_comments WHERE post_id = $1", [post_id])
    comment_id = comment_result[0]["id"]

    # Try to delete post - should fail due to RESTRICT
    with pytest.raises(Exception):
        await execute("DELETE FROM cascade_posts WHERE id = $1", [post_id])

    # Verify nothing was deleted (RESTRICT prevented deletion)
    found_user = await execute("SELECT * FROM cascade_users WHERE id = $1", [user_id])
    assert len(found_user) == 1

    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    assert len(found_post) == 1

    found_comment = await execute("SELECT * FROM cascade_comments WHERE id = $1", [comment_id])
    assert len(found_comment) == 1
