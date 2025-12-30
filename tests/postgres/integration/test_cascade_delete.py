"""Integration tests for cascade-aware delete operations.

This module tests cascade delete operations that handle foreign key relationships
and cascade rules via the PostgreSQL database.
"""

import pytest
from data_bridge.postgres import execute
from data_bridge.test import expect


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
    expect(len(found_user)).to_equal(1)

    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    expect(len(found_post)).to_equal(1)

    # Delete user with cascade (database handles cascade)
    await execute("DELETE FROM cascade_users WHERE id = $1", [user_id])

    # Verify both are deleted (CASCADE handled by database)
    found_user = await execute("SELECT * FROM cascade_users WHERE id = $1", [user_id])
    expect(len(found_user)).to_equal(0)

    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    expect(len(found_post)).to_equal(0)


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

    expect("violate" in str(exc_info.value).lower() or "restrict" in str(exc_info.value).lower()).to_be_true()

    # Verify post still exists
    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    expect(len(found_post)).to_equal(1)


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
    expect(len(found_comment)).to_equal(1)
    expect(found_comment[0]["user_id"]).to_be_none()
    expect(found_comment[0]["text"]).to_equal("Nice post!")


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
    expect(len(found_user)).to_equal(0)

    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    expect(len(found_post)).to_equal(0)


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

    expect("violate" in str(exc_info.value).lower() or "restrict" in str(exc_info.value).lower()).to_be_true()

    # Verify post still exists
    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    expect(len(found_post)).to_equal(1)


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
    expect(len(found_user)).to_equal(0)


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
    expect(len(found_user)).to_equal(0)

    for post_id in post_ids:
        found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
        expect(len(found_post)).to_equal(0)


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
    expect(len(found_user)).to_equal(1)

    found_post = await execute("SELECT * FROM cascade_posts WHERE id = $1", [post_id])
    expect(len(found_post)).to_equal(1)

    found_comment = await execute("SELECT * FROM cascade_comments WHERE id = $1", [comment_id])
    expect(len(found_comment)).to_equal(1)


class TestNestedCascadeDelete:
    """Test nested relationship cascade delete scenarios."""

    @pytest.mark.integration
    @pytest.mark.asyncio
    async def test_cascade_three_levels_deep(self):
        """Test cascade delete through three levels of relationships.

        Schema: User → Post (CASCADE) → Comment (CASCADE) → Reply (CASCADE)
        Delete user, verify all 4 levels cascade.
        """
        try:
            # Create tables
            await execute("""
                CREATE TABLE IF NOT EXISTS nested_users (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL,
                    email TEXT UNIQUE NOT NULL
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS nested_posts (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    FOREIGN KEY (user_id) REFERENCES nested_users(id) ON DELETE CASCADE
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS nested_comments (
                    id SERIAL PRIMARY KEY,
                    post_id INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    FOREIGN KEY (post_id) REFERENCES nested_posts(id) ON DELETE CASCADE
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS nested_replies (
                    id SERIAL PRIMARY KEY,
                    comment_id INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    FOREIGN KEY (comment_id) REFERENCES nested_comments(id) ON DELETE CASCADE
                );
            """)

            # Insert data at all levels
            await execute("""
                INSERT INTO nested_users (name, email)
                VALUES ('John', 'john@example.com')
            """)

            user_result = await execute("SELECT id FROM nested_users WHERE name = 'John'")
            user_id = user_result[0]["id"]

            await execute(f"""
                INSERT INTO nested_posts (user_id, title)
                VALUES ({user_id}, 'My Post')
            """)

            post_result = await execute("SELECT id FROM nested_posts WHERE user_id = $1", [user_id])
            post_id = post_result[0]["id"]

            await execute(f"""
                INSERT INTO nested_comments (post_id, text)
                VALUES ({post_id}, 'Great post!')
            """)

            comment_result = await execute("SELECT id FROM nested_comments WHERE post_id = $1", [post_id])
            comment_id = comment_result[0]["id"]

            await execute(f"""
                INSERT INTO nested_replies (comment_id, text)
                VALUES ({comment_id}, 'I agree!')
            """)

            reply_result = await execute("SELECT id FROM nested_replies WHERE comment_id = $1", [comment_id])
            reply_id = reply_result[0]["id"]

            # Verify all exist
            expect(len(await execute("SELECT * FROM nested_users WHERE id = $1", [user_id]))).to_equal(1)
            expect(len(await execute("SELECT * FROM nested_posts WHERE id = $1", [post_id]))).to_equal(1)
            expect(len(await execute("SELECT * FROM nested_comments WHERE id = $1", [comment_id]))).to_equal(1)
            expect(len(await execute("SELECT * FROM nested_replies WHERE id = $1", [reply_id]))).to_equal(1)

            # Delete user - should cascade through all levels
            await execute("DELETE FROM nested_users WHERE id = $1", [user_id])

            # Verify all are deleted
            expect(len(await execute("SELECT * FROM nested_users WHERE id = $1", [user_id]))).to_equal(0)
            expect(len(await execute("SELECT * FROM nested_posts WHERE id = $1", [post_id]))).to_equal(0)
            expect(len(await execute("SELECT * FROM nested_comments WHERE id = $1", [comment_id]))).to_equal(0)
            expect(len(await execute("SELECT * FROM nested_replies WHERE id = $1", [reply_id]))).to_equal(0)

        finally:
            # Cleanup
            await execute("DROP TABLE IF EXISTS nested_replies CASCADE")
            await execute("DROP TABLE IF EXISTS nested_comments CASCADE")
            await execute("DROP TABLE IF EXISTS nested_posts CASCADE")
            await execute("DROP TABLE IF EXISTS nested_users CASCADE")

    @pytest.mark.integration
    @pytest.mark.asyncio
    async def test_cascade_mixed_rules_nested(self):
        """Test cascade with mixed rules in nested relationships.

        Schema: User (CASCADE) → Post (CASCADE) → Comment (RESTRICT)
        Delete user should fail because nested RESTRICT exists.
        """
        try:
            # Create tables
            await execute("""
                CREATE TABLE IF NOT EXISTS mixed_users (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS mixed_posts (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    FOREIGN KEY (user_id) REFERENCES mixed_users(id) ON DELETE CASCADE
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS mixed_comments (
                    id SERIAL PRIMARY KEY,
                    post_id INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    FOREIGN KEY (post_id) REFERENCES mixed_posts(id) ON DELETE RESTRICT
                );
            """)

            # Insert data
            await execute("INSERT INTO mixed_users (name) VALUES ('Alice')")
            user_result = await execute("SELECT id FROM mixed_users WHERE name = 'Alice'")
            user_id = user_result[0]["id"]

            await execute(f"INSERT INTO mixed_posts (user_id, title) VALUES ({user_id}, 'Post')")
            post_result = await execute("SELECT id FROM mixed_posts WHERE user_id = $1", [user_id])
            post_id = post_result[0]["id"]

            await execute(f"INSERT INTO mixed_comments (post_id, text) VALUES ({post_id}, 'Comment')")

            # Try to delete user - should fail because of nested RESTRICT on comment
            with pytest.raises(Exception) as exc_info:
                await execute("DELETE FROM mixed_users WHERE id = $1", [user_id])

            expect("violate" in str(exc_info.value).lower() or "restrict" in str(exc_info.value).lower()).to_be_true()

            # Verify nothing was deleted
            expect(len(await execute("SELECT * FROM mixed_users WHERE id = $1", [user_id]))).to_equal(1)
            expect(len(await execute("SELECT * FROM mixed_posts WHERE id = $1", [post_id]))).to_equal(1)

        finally:
            # Cleanup
            await execute("DROP TABLE IF EXISTS mixed_comments CASCADE")
            await execute("DROP TABLE IF EXISTS mixed_posts CASCADE")
            await execute("DROP TABLE IF EXISTS mixed_users CASCADE")

    @pytest.mark.integration
    @pytest.mark.asyncio
    async def test_cascade_with_set_null_grandchild(self):
        """Test cascade with SET NULL on grandchild relationship.

        Schema: User (CASCADE) → Post → Comment (SET NULL on optional_user_id)
        Delete user, verify posts cascade but comments remain with NULL user_id.
        """
        try:
            # Create tables
            await execute("""
                CREATE TABLE IF NOT EXISTS setnull_users (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS setnull_posts (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    FOREIGN KEY (user_id) REFERENCES setnull_users(id) ON DELETE CASCADE
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS setnull_comments (
                    id SERIAL PRIMARY KEY,
                    post_id INTEGER NOT NULL,
                    optional_user_id INTEGER,
                    text TEXT NOT NULL,
                    FOREIGN KEY (post_id) REFERENCES setnull_posts(id) ON DELETE CASCADE,
                    FOREIGN KEY (optional_user_id) REFERENCES setnull_users(id) ON DELETE SET NULL
                );
            """)

            # Insert data
            await execute("INSERT INTO setnull_users (name) VALUES ('Bob'), ('Charlie')")
            bob_result = await execute("SELECT id FROM setnull_users WHERE name = 'Bob'")
            bob_id = bob_result[0]["id"]

            charlie_result = await execute("SELECT id FROM setnull_users WHERE name = 'Charlie'")
            charlie_id = charlie_result[0]["id"]

            await execute(f"INSERT INTO setnull_posts (user_id, title) VALUES ({charlie_id}, 'Charlie Post')")
            post_result = await execute("SELECT id FROM setnull_posts WHERE user_id = $1", [charlie_id])
            post_id = post_result[0]["id"]

            # Bob comments on Charlie's post
            await execute(f"""
                INSERT INTO setnull_comments (post_id, optional_user_id, text)
                VALUES ({post_id}, {bob_id}, 'Nice post!')
            """)
            comment_result = await execute("SELECT id FROM setnull_comments WHERE post_id = $1", [post_id])
            comment_id = comment_result[0]["id"]

            # Delete Bob - comment should remain with NULL user_id
            await execute("DELETE FROM setnull_users WHERE id = $1", [bob_id])

            # Verify Bob is deleted
            expect(len(await execute("SELECT * FROM setnull_users WHERE id = $1", [bob_id]))).to_equal(0)

            # Verify comment still exists with NULL user_id
            comment = await execute("SELECT * FROM setnull_comments WHERE id = $1", [comment_id])
            expect(len(comment)).to_equal(1)
            expect(comment[0]["optional_user_id"]).to_be_none()
            expect(comment[0]["text"]).to_equal("Nice post!")

            # Now delete Charlie - post and comment should cascade
            await execute("DELETE FROM setnull_users WHERE id = $1", [charlie_id])

            # Verify post and comment are deleted
            expect(len(await execute("SELECT * FROM setnull_posts WHERE id = $1", [post_id]))).to_equal(0)
            expect(len(await execute("SELECT * FROM setnull_comments WHERE id = $1", [comment_id]))).to_equal(0)

        finally:
            # Cleanup
            await execute("DROP TABLE IF EXISTS setnull_comments CASCADE")
            await execute("DROP TABLE IF EXISTS setnull_posts CASCADE")
            await execute("DROP TABLE IF EXISTS setnull_users CASCADE")

    @pytest.mark.integration
    @pytest.mark.asyncio
    async def test_cascade_sibling_branches(self):
        """Test cascade delete with sibling branches.

        Schema: User (CASCADE) → Post and User (CASCADE) → Profile
        Delete user, verify both branches cascade independently.
        """
        try:
            # Create tables
            await execute("""
                CREATE TABLE IF NOT EXISTS branch_users (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS branch_posts (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    FOREIGN KEY (user_id) REFERENCES branch_users(id) ON DELETE CASCADE
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS branch_profiles (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    bio TEXT,
                    FOREIGN KEY (user_id) REFERENCES branch_users(id) ON DELETE CASCADE
                );
            """)

            # Insert data
            await execute("INSERT INTO branch_users (name) VALUES ('Diana')")
            user_result = await execute("SELECT id FROM branch_users WHERE name = 'Diana'")
            user_id = user_result[0]["id"]

            await execute(f"INSERT INTO branch_posts (user_id, title) VALUES ({user_id}, 'Post 1'), ({user_id}, 'Post 2')")
            post_results = await execute("SELECT id FROM branch_posts WHERE user_id = $1", [user_id])
            post_ids = [row["id"] for row in post_results]

            await execute(f"INSERT INTO branch_profiles (user_id, bio) VALUES ({user_id}, 'Hello world')")
            profile_result = await execute("SELECT id FROM branch_profiles WHERE user_id = $1", [user_id])
            profile_id = profile_result[0]["id"]

            # Verify all exist
            expect(len(await execute("SELECT * FROM branch_users WHERE id = $1", [user_id]))).to_equal(1)
            expect(len(post_results)).to_equal(2)
            expect(len(await execute("SELECT * FROM branch_profiles WHERE id = $1", [profile_id]))).to_equal(1)

            # Delete user - both branches should cascade
            await execute("DELETE FROM branch_users WHERE id = $1", [user_id])

            # Verify user is deleted
            expect(len(await execute("SELECT * FROM branch_users WHERE id = $1", [user_id]))).to_equal(0)

            # Verify all posts are deleted
            for post_id in post_ids:
                expect(len(await execute("SELECT * FROM branch_posts WHERE id = $1", [post_id]))).to_equal(0)

            # Verify profile is deleted
            expect(len(await execute("SELECT * FROM branch_profiles WHERE id = $1", [profile_id]))).to_equal(0)

        finally:
            # Cleanup
            await execute("DROP TABLE IF EXISTS branch_profiles CASCADE")
            await execute("DROP TABLE IF EXISTS branch_posts CASCADE")
            await execute("DROP TABLE IF EXISTS branch_users CASCADE")

    @pytest.mark.integration
    @pytest.mark.asyncio
    async def test_cascade_diamond_relationship(self):
        """Test cascade with diamond relationship.

        Schema: User → Post (CASCADE), User → Comment (CASCADE), Post → Comment (RESTRICT)
        Both Post and Comment reference User, Comment also references Post.
        Test cascade behavior with shared parent.
        """
        try:
            # Create tables
            await execute("""
                CREATE TABLE IF NOT EXISTS diamond_users (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS diamond_posts (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    FOREIGN KEY (user_id) REFERENCES diamond_users(id) ON DELETE CASCADE
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS diamond_comments (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    post_id INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    FOREIGN KEY (user_id) REFERENCES diamond_users(id) ON DELETE CASCADE,
                    FOREIGN KEY (post_id) REFERENCES diamond_posts(id) ON DELETE RESTRICT
                );
            """)

            # Insert data
            await execute("INSERT INTO diamond_users (name) VALUES ('Eve')")
            user_result = await execute("SELECT id FROM diamond_users WHERE name = 'Eve'")
            user_id = user_result[0]["id"]

            await execute(f"INSERT INTO diamond_posts (user_id, title) VALUES ({user_id}, 'Eve Post')")
            post_result = await execute("SELECT id FROM diamond_posts WHERE user_id = $1", [user_id])
            post_id = post_result[0]["id"]

            await execute(f"INSERT INTO diamond_comments (user_id, post_id, text) VALUES ({user_id}, {post_id}, 'Comment')")

            # Try to delete user - should fail because comment has RESTRICT on post
            with pytest.raises(Exception) as exc_info:
                await execute("DELETE FROM diamond_users WHERE id = $1", [user_id])

            expect("violate" in str(exc_info.value).lower() or "restrict" in str(exc_info.value).lower()).to_be_true()

            # Verify nothing was deleted
            expect(len(await execute("SELECT * FROM diamond_users WHERE id = $1", [user_id]))).to_equal(1)
            expect(len(await execute("SELECT * FROM diamond_posts WHERE id = $1", [post_id]))).to_equal(1)

            # Delete comment first, then user should cascade successfully
            await execute(f"DELETE FROM diamond_comments WHERE post_id = {post_id}")
            await execute("DELETE FROM diamond_users WHERE id = $1", [user_id])

            # Verify cascade worked
            expect(len(await execute("SELECT * FROM diamond_users WHERE id = $1", [user_id]))).to_equal(0)
            expect(len(await execute("SELECT * FROM diamond_posts WHERE id = $1", [post_id]))).to_equal(0)

        finally:
            # Cleanup
            await execute("DROP TABLE IF EXISTS diamond_comments CASCADE")
            await execute("DROP TABLE IF EXISTS diamond_posts CASCADE")
            await execute("DROP TABLE IF EXISTS diamond_users CASCADE")

    @pytest.mark.integration
    @pytest.mark.asyncio
    async def test_cascade_large_nested_tree(self):
        """Test cascade with large nested tree.

        Create: 1 user → 5 posts → 10 comments each = 50 comments
        Delete user, verify all cascade efficiently.
        """
        try:
            # Create tables
            await execute("""
                CREATE TABLE IF NOT EXISTS large_users (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS large_posts (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    FOREIGN KEY (user_id) REFERENCES large_users(id) ON DELETE CASCADE
                );
            """)

            await execute("""
                CREATE TABLE IF NOT EXISTS large_comments (
                    id SERIAL PRIMARY KEY,
                    post_id INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    FOREIGN KEY (post_id) REFERENCES large_posts(id) ON DELETE CASCADE
                );
            """)

            # Insert user
            await execute("INSERT INTO large_users (name) VALUES ('Frank')")
            user_result = await execute("SELECT id FROM large_users WHERE name = 'Frank'")
            user_id = user_result[0]["id"]

            # Insert 5 posts
            post_ids = []
            for i in range(5):
                await execute(f"INSERT INTO large_posts (user_id, title) VALUES ({user_id}, 'Post {i+1}')")

            post_results = await execute("SELECT id FROM large_posts WHERE user_id = $1", [user_id])
            post_ids = [row["id"] for row in post_results]
            expect(len(post_ids)).to_equal(5)

            # Insert 10 comments per post (50 total)
            comment_count = 0
            for post_id in post_ids:
                for j in range(10):
                    await execute(f"INSERT INTO large_comments (post_id, text) VALUES ({post_id}, 'Comment {j+1}')")
                    comment_count += 1

            # Verify all comments exist
            all_comments = await execute("SELECT COUNT(*) as cnt FROM large_comments")
            expect(all_comments[0]["cnt"]).to_equal(50)

            # Delete user - should cascade through all posts and comments
            await execute("DELETE FROM large_users WHERE id = $1", [user_id])

            # Verify all are deleted
            expect(len(await execute("SELECT * FROM large_users WHERE id = $1", [user_id]))).to_equal(0)

            remaining_posts = await execute("SELECT COUNT(*) as cnt FROM large_posts WHERE user_id = $1", [user_id])
            expect(remaining_posts[0]["cnt"]).to_equal(0)

            remaining_comments = await execute("SELECT COUNT(*) as cnt FROM large_comments")
            expect(remaining_comments[0]["cnt"]).to_equal(0)

        finally:
            # Cleanup
            await execute("DROP TABLE IF EXISTS large_comments CASCADE")
            await execute("DROP TABLE IF EXISTS large_posts CASCADE")
            await execute("DROP TABLE IF EXISTS large_users CASCADE")
