"""
Integration tests for lazy loading relationships (Phase 2).

Tests Phase 2 implementation:
- SELECT strategy (separate query)
- NULL foreign key handling
- Caching loaded values
- Session identity map integration

Prerequisites:
- PostgreSQL container 'rstn-postgres' running on port 5432
- Database credentials: rstn:rstn
- Test database: ouroboros_test

Run with:
    POSTGRES_URI="postgresql://rstn:rstn@localhost:5432/ouroboros_test" \
        uv run pytest tests/postgres/integration/test_lazy_loading.py -v

Setup database:
    bash scripts/setup_test_db.sh
"""
import pytest
from ouroboros.qc import expect
from ouroboros.postgres import Table, Column, relationship, init
from ouroboros.postgres.session import Session


# Test models
class User(Table):
    """User model for testing."""
    id: int = Column(primary_key=True)
    name: str

    class Settings:
        table_name = "test_users_lazy"


class Post(Table):
    """Post model with relationship to User."""
    id: int = Column(primary_key=True)
    title: str
    author_id: int = Column(foreign_key="test_users_lazy.id")

    author: User = relationship(User, foreign_key_column="author_id")

    class Settings:
        table_name = "test_posts_lazy"


@pytest.fixture(scope="function")
async def setup_tables():
    """Setup test tables (database already initialized by conftest)."""
    # Create tables
    await User.create_table()
    await Post.create_table()

    yield

    # Cleanup will be handled by cleanup_tables fixture in conftest


@pytest.fixture
async def sample_data(setup_tables):
    """Insert sample data (depends on setup_tables)."""
    # Clear existing data
    await Post.delete_many({})
    await User.delete_many({})

    # Insert users
    user1 = User(id=1, name="Alice")
    await user1.save()

    user2 = User(id=2, name="Bob")
    await user2.save()

    # Insert posts
    post1 = Post(id=1, title="Post 1", author_id=1)
    await post1.save()

    post2 = Post(id=2, title="Post 2", author_id=2)
    await post2.save()

    post3 = Post(id=3, title="Post 3", author_id=None)
    await post3.save()

    yield

    # Cleanup (tables will be dropped by cleanup_tables fixture)


@pytest.mark.asyncio
async def test_lazy_load_select_strategy(sample_data):
    """Test lazy loading with separate SELECT query."""
    post = await Post.get(1)

    # Should not be loaded yet
    assert not post.author.is_loaded

    # Load author
    author = await post.author

    # Should be loaded now
    assert post.author.is_loaded
    assert author is not None
    assert author.name == "Alice"
    assert author.id == 1


@pytest.mark.asyncio
async def test_lazy_load_caching(sample_data):
    """Test that loaded values are cached."""
    post = await Post.get(1)

    # First load
    author1 = await post.author

    # Second access should return cached value
    author2 = await post.author

    # Should be same instance
    assert author1 is author2


@pytest.mark.asyncio
async def test_lazy_load_null_fk(sample_data):
    """Test that NULL foreign keys return None."""
    post = await Post.get(3)

    # FK is None
    assert post.author.ref is None

    # Load should return None
    author = await post.author
    assert author is None

    # Should be marked as loaded
    assert post.author.is_loaded


@pytest.mark.asyncio
async def test_lazy_load_with_session(sample_data):
    """Test lazy loading works with session."""
    async with Session() as session:
        post = await session.get(Post, 1)

        # Load author through session
        author = await post.author

        assert author is not None
        assert author.name == "Alice"


@pytest.mark.asyncio
async def test_lazy_load_identity_map(sample_data):
    """Test identity map ensures same instance."""
    async with Session() as session:
        # Get two posts with same author
        post1 = await session.get(Post, 1)
        post2 = await session.get(Post, 1)  # Same post

        # Both should reference same instance
        author1 = await post1.author
        author2 = await post2.author

        # Should be same instance from identity map
        assert author1 is author2


@pytest.mark.asyncio
async def test_lazy_load_identity_map_different_posts(sample_data):
    """Test identity map with different posts referencing same user."""
    async with Session() as session:
        # Update post2 to have same author as post1
        post2 = await session.get(Post, 2)
        post2.author_id = 1
        await session.commit()

        # Load both posts
        post1 = await session.get(Post, 1)
        post2_reloaded = await session.get(Post, 2)

        # Load authors
        author1 = await post1.author
        author2 = await post2_reloaded.author

        # Should be same instance from identity map
        assert author1 is author2
        assert author1.id == 1
        assert author1.name == "Alice"


@pytest.mark.asyncio
async def test_lazy_load_ref_without_loading(sample_data):
    """Test that ref property accesses FK without loading."""
    post = await Post.get(1)

    # Get FK without loading
    author_id = post.author.ref
    assert author_id == 1

    # Should not be loaded
    assert not post.author.is_loaded


@pytest.mark.asyncio
async def test_lazy_load_multiple_posts(sample_data):
    """Test lazy loading on multiple posts."""
    post1 = await Post.get(1)
    post2 = await Post.get(2)

    # Load both authors
    author1 = await post1.author
    author2 = await post2.author

    # Should be different authors
    assert author1.id == 1
    assert author1.name == "Alice"
    assert author2.id == 2
    assert author2.name == "Bob"

    # Both should be loaded
    assert post1.author.is_loaded
    assert post2.author.is_loaded


@pytest.mark.asyncio
async def test_lazy_load_without_session(sample_data):
    """Test lazy loading works without session context."""
    post = await Post.get(1)

    # Load author (should use standalone query)
    author = await post.author

    assert author is not None
    assert author.name == "Alice"
    assert post.author.is_loaded


@pytest.mark.asyncio
async def test_lazy_load_descriptor_class_access(sample_data):
    """Test that descriptor returns itself on class access."""
    # Class access should return descriptor
    descriptor = Post.author

    # Should be a RelationshipDescriptor
    from ouroboros.postgres.relationships import RelationshipDescriptor
    assert isinstance(descriptor, RelationshipDescriptor)


@pytest.mark.asyncio
async def test_lazy_load_descriptor_instance_access(sample_data):
    """Test that descriptor returns loader on instance access."""
    post = await Post.get(1)

    # Instance access should return loader
    loader = post.author

    # Should be a RelationshipLoader
    from ouroboros.postgres.relationships import RelationshipLoader
    assert isinstance(loader, RelationshipLoader)


@pytest.mark.asyncio
async def test_lazy_load_nonexistent_fk(sample_data):
    """Test lazy loading with FK pointing to nonexistent record."""
    # Create post with FK to nonexistent user
    post = Post(id=999, title="Orphan Post", author_id=999)
    await post.save()

    try:
        # Load author (should return None)
        author = await post.author
        assert author is None
        assert post.author.is_loaded
    finally:
        # Cleanup
        await Post.delete_many({"id": 999})


# ============================================================================
# Phase 3: Eager Loading Tests (selectinload, joinedload, noload)
# ============================================================================


@pytest.mark.asyncio
async def test_selectinload_prevents_n_plus_1(sample_data):
    """Selectinload batch loads relationships to prevent N+1."""
    # Clear and insert test data
    await User.delete_many({})
    await Post.delete_many({})

    # Insert 10 users
    for i in range(1, 11):
        await User(id=i, name=f"User {i}").save()

    # Insert 10 posts, each with different author
    for i in range(1, 11):
        await Post(id=i, title=f"Post {i}", author_id=i).save()

    # Load posts with selectinload
    from ouroboros.postgres import selectinload

    posts = await Post.find().options(selectinload("author")).to_list()

    # All authors should already be loaded (2 queries total: posts + authors)
    for post in posts:
        # Should not trigger additional query
        author = await post.author
        assert author is not None
        assert post.author.is_loaded
        assert author.name == f"User {post.id}"


@pytest.mark.asyncio
async def test_selectinload_with_null_fk(sample_data):
    """Selectinload handles NULL FKs correctly."""
    await User.delete_many({})
    await Post.delete_many({})

    await User(id=1, name="Alice").save()
    await Post(id=1, title="Post 1", author_id=1).save()
    await Post(id=2, title="Post 2", author_id=None).save()  # NULL FK

    from ouroboros.postgres import selectinload

    posts = await Post.find().options(selectinload("author")).to_list()

    # First post should have author
    author1 = await posts[0].author
    assert author1 is not None
    assert author1.name == "Alice"

    # Second post should have None
    author2 = await posts[1].author
    assert author2 is None


@pytest.mark.asyncio
async def test_selectinload_multiple_posts_same_author(sample_data):
    """Selectinload handles multiple posts with same author."""
    await User.delete_many({})
    await Post.delete_many({})

    await User(id=1, name="Alice").save()
    await Post(id=1, title="Post 1", author_id=1).save()
    await Post(id=2, title="Post 2", author_id=1).save()
    await Post(id=3, title="Post 3", author_id=1).save()

    from ouroboros.postgres import selectinload

    posts = await Post.find().options(selectinload("author")).to_list()

    # All should load the same author
    authors = []
    for post in posts:
        author = await post.author
        authors.append(author)

    # Should all be loaded
    assert all(author is not None for author in authors)
    assert all(author.name == "Alice" for author in authors)
    assert all(author.id == 1 for author in authors)


@pytest.mark.asyncio
async def test_noload_option(sample_data):
    """NoLoad option marks relationship as never loaded."""
    await User.delete_many({})
    await Post.delete_many({})

    await User(id=1, name="Alice").save()
    await Post(id=1, title="Post 1", author_id=1).save()

    from ouroboros.postgres import noload

    posts = await Post.find().options(noload("author")).to_list()

    # Should be marked as loaded with None
    post = posts[0]
    assert post.author.is_loaded
    author = await post.author
    assert author is None  # NoLoad returns None


@pytest.mark.asyncio
async def test_multiple_options(sample_data):
    """Multiple options can be applied."""
    await User.delete_many({})
    await Post.delete_many({})

    await User(id=1, name="Alice").save()
    await Post(id=1, title="Post 1", author_id=1).save()

    # This test is just to verify syntax works
    # In real scenario, you'd load multiple relationships
    from ouroboros.postgres import selectinload

    posts = await Post.find().options(
        selectinload("author"),
        # selectinload("comments"),  # If we had this relationship
    ).to_list()

    assert posts is not None
    assert len(posts) == 1

    # Verify author was loaded
    author = await posts[0].author
    assert author is not None
    assert author.name == "Alice"


@pytest.mark.asyncio
async def test_selectinload_with_empty_result(sample_data):
    """Selectinload works with empty query results."""
    await Post.delete_many({})

    from ouroboros.postgres import selectinload

    # Query with no results
    posts = await Post.find(Post.id == 999).options(selectinload("author")).to_list()

    assert posts == []


@pytest.mark.asyncio
async def test_selectinload_invalid_relationship(sample_data):
    """Selectinload raises error for invalid relationship name."""
    await Post.delete_many({})
    await Post(id=1, title="Post 1", author_id=1).save()

    from ouroboros.postgres import selectinload

    # Error should be raised when applying option during to_list()
    expect(lambda: posts = await Post.find().options(selectinload("invalid_relationship")).to_list()).to_raise(ValueError)


@pytest.mark.asyncio
async def test_selectinload_with_all_null_fks(sample_data):
    """Selectinload handles case where all FKs are NULL."""
    await User.delete_many({})
    await Post.delete_many({})

    # Create posts with NULL author_id
    await Post(id=1, title="Post 1", author_id=None).save()
    await Post(id=2, title="Post 2", author_id=None).save()

    from ouroboros.postgres import selectinload

    posts = await Post.find().options(selectinload("author")).to_list()

    # All should have None as author
    for post in posts:
        author = await post.author
        assert author is None
        assert post.author.is_loaded


@pytest.mark.asyncio
async def test_joinedload_not_implemented(sample_data):
    """JoinedLoad raises NotImplementedError."""
    await Post.delete_many({})
    await Post(id=1, title="Post 1", author_id=1).save()

    from ouroboros.postgres import joinedload

    # Should raise NotImplementedError when trying to apply during to_list()
    expect(lambda: posts = await Post.find().options(joinedload("author")).to_list()).to_raise(NotImplementedError)


@pytest.mark.asyncio
async def test_selectinload_chaining_with_filters(sample_data):
    """Selectinload works with filtered queries."""
    await User.delete_many({})
    await Post.delete_many({})

    await User(id=1, name="Alice").save()
    await User(id=2, name="Bob").save()
    await Post(id=1, title="Post 1", author_id=1).save()
    await Post(id=2, title="Post 2", author_id=2).save()

    from ouroboros.postgres import selectinload

    # Filter posts and eagerly load authors
    posts = await Post.find(Post.id == 1).options(selectinload("author")).to_list()

    assert len(posts) == 1
    author = await posts[0].author
    assert author is not None
    assert author.name == "Alice"


@pytest.mark.asyncio
async def test_selectinload_with_order_by(sample_data):
    """Selectinload works with ordered queries."""
    await User.delete_many({})
    await Post.delete_many({})

    await User(id=1, name="Alice").save()
    await User(id=2, name="Bob").save()
    await Post(id=1, title="Post A", author_id=1).save()
    await Post(id=2, title="Post B", author_id=2).save()

    from ouroboros.postgres import selectinload

    # Order posts and eagerly load authors
    posts = await Post.find().order_by("-id").options(selectinload("author")).to_list()

    assert len(posts) == 2
    assert posts[0].id == 2  # Ordered descending
    assert posts[1].id == 1

    # Verify authors loaded
    author1 = await posts[0].author
    author2 = await posts[1].author
    assert author1.name == "Bob"
    assert author2.name == "Alice"


@pytest.mark.asyncio
async def test_selectinload_with_limit(sample_data):
    """Selectinload works with limited queries."""
    await User.delete_many({})
    await Post.delete_many({})

    for i in range(1, 6):
        await User(id=i, name=f"User {i}").save()
        await Post(id=i, title=f"Post {i}", author_id=i).save()

    from ouroboros.postgres import selectinload

    # Limit to 3 posts
    posts = await Post.find().limit(3).options(selectinload("author")).to_list()

    assert len(posts) == 3

    # All authors should be loaded
    for post in posts:
        author = await post.author
        assert author is not None
        assert post.author.is_loaded
