"""
Integration tests for Many-to-Many relationship functionality.

Tests M2M operations with real PostgreSQL database.
"""
import pytest

try:
    from data_bridge.data_bridge import postgres as _engine
except ImportError:
    _engine = None


@pytest.fixture
async def m2m_tables(request):
    """Create posts and tags tables with a join table for M2M testing."""
    # Import execute from connection module
    from data_bridge.postgres import execute, insert_one

    # Create posts table
    await execute("""
        CREATE TABLE IF NOT EXISTS posts (
            id SERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            content TEXT
        )
    """)

    # Create tags table
    await execute("""
        CREATE TABLE IF NOT EXISTS tags (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100) NOT NULL UNIQUE
        )
    """)

    # Create join table
    await execute("""
        CREATE TABLE IF NOT EXISTS post_tags (
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (post_id, tag_id)
        )
    """)

    # Insert test posts
    posts = [
        {"title": "First Post", "content": "Content 1"},
        {"title": "Second Post", "content": "Content 2"},
        {"title": "Third Post", "content": "Content 3"},
    ]
    post_ids = []
    for post in posts:
        result = await insert_one("posts", post)
        post_ids.append(result["id"])

    # Insert test tags
    tags = [
        {"name": "python"},
        {"name": "rust"},
        {"name": "database"},
        {"name": "orm"},
    ]
    tag_ids = []
    for tag in tags:
        result = await insert_one("tags", tag)
        tag_ids.append(result["id"])

    yield {"post_ids": post_ids, "tag_ids": tag_ids}

    # Cleanup is handled by cleanup_tables fixture in conftest.py


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MAddRelation:
    """Test adding M2M relations."""

    async def test_add_single_relation(self, m2m_tables):
        """Test adding a single M2M relation."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_id = m2m_tables["tag_ids"][0]

        await _engine.m2m_add_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )

        # Verify relation exists
        exists = await _engine.m2m_has_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )
        assert exists is True

    async def test_add_multiple_relations(self, m2m_tables):
        """Test adding multiple M2M relations."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_ids = m2m_tables["tag_ids"][:3]

        for tag_id in tag_ids:
            await _engine.m2m_add_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )

        count = await _engine.m2m_count_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, "id", "id"
        )
        assert count == 3

    async def test_add_duplicate_relation_idempotent(self, m2m_tables):
        """Test that adding duplicate relation is idempotent."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_id = m2m_tables["tag_ids"][0]

        # Add twice
        await _engine.m2m_add_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )
        await _engine.m2m_add_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )

        # Should still have only 1 relation
        count = await _engine.m2m_count_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, "id", "id"
        )
        assert count == 1


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MRemoveRelation:
    """Test removing M2M relations."""

    async def test_remove_single_relation(self, m2m_tables):
        """Test removing a single M2M relation."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_id = m2m_tables["tag_ids"][0]

        # Add then remove
        await _engine.m2m_add_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )
        affected = await _engine.m2m_remove_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )

        assert affected == 1
        exists = await _engine.m2m_has_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )
        assert exists is False

    async def test_remove_nonexistent_relation(self, m2m_tables):
        """Test removing a nonexistent relation returns 0."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_id = m2m_tables["tag_ids"][0]

        affected = await _engine.m2m_remove_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )
        assert affected == 0


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MClearRelations:
    """Test clearing all M2M relations."""

    async def test_clear_all_relations(self, m2m_tables):
        """Test clearing all relations for a source."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_ids = m2m_tables["tag_ids"]

        # Add all tags
        for tag_id in tag_ids:
            await _engine.m2m_add_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )

        # Clear all
        affected = await _engine.m2m_clear_relations(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, "id", "id"
        )

        assert affected == len(tag_ids)
        count = await _engine.m2m_count_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, "id", "id"
        )
        assert count == 0


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MFetchRelated:
    """Test fetching M2M related records."""

    async def test_fetch_all_related(self, m2m_tables):
        """Test fetching all related records."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_ids = m2m_tables["tag_ids"][:2]

        for tag_id in tag_ids:
            await _engine.m2m_add_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )

        results = await _engine.m2m_fetch_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, None, None, None, "id", "id"
        )

        assert len(results) == 2
        names = [r["name"] for r in results]
        assert "python" in names
        assert "rust" in names

    async def test_fetch_with_select_columns(self, m2m_tables):
        """Test fetching with specific columns."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_id = m2m_tables["tag_ids"][0]

        await _engine.m2m_add_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )

        results = await _engine.m2m_fetch_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, ["name"], None, None, "id", "id"
        )

        assert len(results) == 1
        assert "name" in results[0]

    async def test_fetch_with_order_by(self, m2m_tables):
        """Test fetching with ordering."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_ids = m2m_tables["tag_ids"]

        for tag_id in tag_ids:
            await _engine.m2m_add_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )

        results = await _engine.m2m_fetch_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, None, [("name", "asc")], None, "id", "id"
        )

        names = [r["name"] for r in results]
        assert names == sorted(names)

    async def test_fetch_with_limit(self, m2m_tables):
        """Test fetching with limit."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_ids = m2m_tables["tag_ids"]

        for tag_id in tag_ids:
            await _engine.m2m_add_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )

        results = await _engine.m2m_fetch_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, None, None, 2, "id", "id"
        )

        assert len(results) == 2


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MSetRelations:
    """Test setting M2M relations atomically."""

    async def test_set_replaces_existing(self, m2m_tables):
        """Test that set replaces all existing relations."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        old_tags = m2m_tables["tag_ids"][:2]
        new_tags = m2m_tables["tag_ids"][2:]

        # Add old tags
        for tag_id in old_tags:
            await _engine.m2m_add_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )

        # Set new tags
        await _engine.m2m_set_relations(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, new_tags, "id", "id"
        )

        # Verify old tags removed
        for tag_id in old_tags:
            exists = await _engine.m2m_has_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )
            assert exists is False

        # Verify new tags added
        for tag_id in new_tags:
            exists = await _engine.m2m_has_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )
            assert exists is True

    async def test_set_empty_clears_all(self, m2m_tables):
        """Test that setting empty list clears all relations."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_ids = m2m_tables["tag_ids"]

        # Add all tags
        for tag_id in tag_ids:
            await _engine.m2m_add_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )

        # Set to empty list
        await _engine.m2m_set_relations(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, [], "id", "id"
        )

        count = await _engine.m2m_count_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, "id", "id"
        )
        assert count == 0


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MCreateJoinTable:
    """Test automatic join table creation."""

    async def test_create_join_table(self):
        """Test creating a join table automatically."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        from data_bridge.postgres import execute

        # Create source and target tables first
        await execute("""
            CREATE TABLE IF NOT EXISTS users_test (
                id SERIAL PRIMARY KEY,
                name VARCHAR(100)
            )
        """)
        await execute("""
            CREATE TABLE IF NOT EXISTS groups_test (
                id SERIAL PRIMARY KEY,
                name VARCHAR(100)
            )
        """)

        # Create join table
        await _engine.m2m_create_join_table(
            "user_groups_test",
            "user_id", "group_id",
            "users_test", "groups_test",
            "id", "id"
        )

        # Verify table exists by inserting
        await execute("INSERT INTO users_test (name) VALUES ('Alice')")
        await execute("INSERT INTO groups_test (name) VALUES ('Admins')")
        await execute("INSERT INTO user_groups_test (user_id, group_id) VALUES (1, 1)")

        # Verify foreign key constraint
        result = await execute("SELECT * FROM user_groups_test")
        assert len(result) == 1


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MCountRelated:
    """Test counting M2M relations."""

    async def test_count_zero_relations(self, m2m_tables):
        """Test counting when there are no relations."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]

        count = await _engine.m2m_count_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, "id", "id"
        )
        assert count == 0

    async def test_count_multiple_relations(self, m2m_tables):
        """Test counting multiple relations."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_ids = m2m_tables["tag_ids"]

        # Add all tags
        for tag_id in tag_ids:
            await _engine.m2m_add_relation(
                "post_tags", "post_id", "tag_id", "tags",
                post_id, tag_id, "id", "id"
            )

        count = await _engine.m2m_count_related(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, "id", "id"
        )
        assert count == len(tag_ids)


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MHasRelation:
    """Test checking existence of M2M relations."""

    async def test_has_existing_relation(self, m2m_tables):
        """Test checking for existing relation."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_id = m2m_tables["tag_ids"][0]

        # Add relation
        await _engine.m2m_add_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )

        exists = await _engine.m2m_has_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )
        assert exists is True

    async def test_has_nonexistent_relation(self, m2m_tables):
        """Test checking for nonexistent relation."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_id = m2m_tables["tag_ids"][0]

        exists = await _engine.m2m_has_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )
        assert exists is False


@pytest.mark.integration
@pytest.mark.asyncio
class TestM2MBidirectional:
    """Test bidirectional M2M relationships."""

    async def test_bidirectional_relations(self, m2m_tables):
        """Test that relations work in both directions."""
        if _engine is None:
            pytest.skip("Rust engine not available")

        post_id = m2m_tables["post_ids"][0]
        tag_id = m2m_tables["tag_ids"][0]

        # Add relation from post to tag
        await _engine.m2m_add_relation(
            "post_tags", "post_id", "tag_id", "tags",
            post_id, tag_id, "id", "id"
        )

        # Verify from tag to post
        results = await _engine.m2m_fetch_related(
            "post_tags", "tag_id", "post_id", "posts",
            tag_id, None, None, None, "id", "id"
        )

        assert len(results) == 1
        assert results[0]["id"] == post_id
        assert results[0]["title"] == "First Post"
