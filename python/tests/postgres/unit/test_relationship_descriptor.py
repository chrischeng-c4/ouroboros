"""Unit tests for relationship descriptors.

Tests the descriptor protocol and relationship registration without
requiring a database connection.
"""

import pytest
from ouroboros.test import expect

from ouroboros.postgres import Table, Column, relationship
from ouroboros.postgres.relationships import (
    RelationshipDescriptor,
    RelationshipLoader,
    LoadingStrategy,
)


# Test models
class User(Table):
    """User model for testing."""

    id: int = Column(primary_key=True)
    name: str

    class Settings:
        table_name = "users"


class Post(Table):
    """Post model for testing."""

    id: int = Column(primary_key=True)
    title: str
    author_id: int = Column(foreign_key="users.id")

    # Define relationship
    author: User = relationship(User, foreign_key_column="author_id")

    class Settings:
        table_name = "posts"


class Comment(Table):
    """Comment model for testing with different strategies."""

    id: int = Column(primary_key=True)
    content: str
    post_id: int = Column(foreign_key="posts.id")
    author_id: int = Column(foreign_key="users.id")

    # Test different loading strategies
    post: Post = relationship(Post, foreign_key_column="post_id", lazy="joined")
    author: User = relationship(User, foreign_key_column="author_id", lazy="selectinload")

    class Settings:
        table_name = "comments"


# Tests


def test_relationship_descriptor_class_access():
    """Descriptor returns self on class access."""
    # Class access should return the descriptor itself
    assert isinstance(Post.author, RelationshipDescriptor)
    assert Post.author._target_model is User
    assert Post.author._foreign_key_column == "author_id"
    assert Post.author._lazy == "select"


def test_relationship_loader_instance_access():
    """Descriptor returns loader on instance access."""
    # Create instance
    post = Post(id=1, title="Test Post", author_id=123)

    # Instance access should return a loader
    loader = post.author
    assert isinstance(loader, RelationshipLoader)
    assert loader._instance is post
    assert loader._descriptor is Post.author


def test_relationship_loader_ref():
    """Can access FK value without loading."""
    # Create instance with FK value
    post = Post(id=1, title="Test Post", author_id=456)

    # Access FK value via .ref
    assert post.author.ref == 456

    # Should work even if FK is None
    post_no_author = Post(id=2, title="No Author", author_id=None)
    assert post_no_author.author.ref is None


def test_relationship_loader_is_loaded():
    """is_loaded tracks loading state."""
    post = Post(id=1, title="Test Post", author_id=123)

    # Initially not loaded
    assert not post.author.is_loaded

    # After accessing (even without loading), still not loaded in Phase 1
    loader = post.author
    assert not loader.is_loaded


@pytest.mark.asyncio
async def test_relationship_loader_not_implemented():
    """Loading raises NotImplementedError in Phase 1."""
    post = Post(id=1, title="Test Post", author_id=123)

    # Attempting to await the loader should raise NotImplementedError
    exc_info = expect(lambda: await post.author).to_raise(NotImplementedError)

    # Check error message
    assert "Lazy loading not yet implemented" in str(exc_info.value)
    assert "Phase 2" in str(exc_info.value)
    assert "author" in str(exc_info.value)
    assert "Post" in str(exc_info.value)


def test_tablemeta_registers_relationships():
    """TableMeta registers relationships in _relationships dict."""
    # Check that Post has _relationships attribute
    assert hasattr(Post, "_relationships")
    assert isinstance(Post._relationships, dict)

    # Check that 'author' relationship is registered
    assert "author" in Post._relationships
    assert Post._relationships["author"] is Post.author

    # Check Comment has multiple relationships registered
    assert hasattr(Comment, "_relationships")
    assert "post" in Comment._relationships
    assert "author" in Comment._relationships
    assert Comment._relationships["post"] is Comment.post
    assert Comment._relationships["author"] is Comment.author


def test_relationship_descriptor_set_name():
    """__set_name__ is called and sets the name."""
    # The descriptor should have its name set
    assert Post.author._name == "author"
    assert Comment.post._name == "post"
    assert Comment.author._name == "author"


def test_relationship_loading_strategies():
    """Different loading strategies are stored correctly."""
    # Default strategy
    assert Post.author._lazy == "select"

    # Custom strategies
    assert Comment.post._lazy == "joined"
    assert Comment.author._lazy == "selectinload"


def test_relationship_factory_function():
    """relationship() factory function creates descriptor."""
    descriptor = relationship(
        User,
        foreign_key_column="user_id",
        lazy="joined",
        back_populates="posts",
        uselist=True,
    )

    assert isinstance(descriptor, RelationshipDescriptor)
    assert descriptor._target_model is User
    assert descriptor._foreign_key_column == "user_id"
    assert descriptor._lazy == "joined"
    assert descriptor._back_populates == "posts"
    assert descriptor._uselist is True


def test_multiple_instances_separate_loaders():
    """Each instance gets its own loader."""
    post1 = Post(id=1, title="Post 1", author_id=100)
    post2 = Post(id=2, title="Post 2", author_id=200)

    loader1 = post1.author
    loader2 = post2.author

    # Different loader instances
    assert loader1 is not loader2

    # But same descriptor
    assert loader1._descriptor is loader2._descriptor
    assert loader1._descriptor is Post.author

    # Different instances
    assert loader1._instance is post1
    assert loader2._instance is post2

    # Different FK values
    assert loader1.ref == 100
    assert loader2.ref == 200


def test_loading_strategy_enum():
    """LoadingStrategy enum has all expected values."""
    assert LoadingStrategy.SELECT.value == "select"
    assert LoadingStrategy.JOINED.value == "joined"
    assert LoadingStrategy.SUBQUERY.value == "subquery"
    assert LoadingStrategy.SELECTIN.value == "selectinload"
    assert LoadingStrategy.NOLOAD.value == "noload"
    assert LoadingStrategy.RAISE.value == "raise"


def test_relationship_descriptor_without_annotation():
    """Relationships can be defined without type annotation."""

    class Article(Table):
        id: int = Column(primary_key=True)
        author_id: int

        # Define relationship without type annotation
        author = relationship(User, foreign_key_column="author_id")

        class Settings:
            table_name = "articles"

    # Should still work
    assert isinstance(Article.author, RelationshipDescriptor)
    assert "author" in Article._relationships


def test_relationship_back_populates():
    """back_populates is stored correctly."""

    class Author(Table):
        id: int = Column(primary_key=True)
        name: str

        posts = relationship(
            Post, foreign_key_column="author_id", back_populates="author", uselist=True
        )

        class Settings:
            table_name = "authors"

    assert Author.posts._back_populates == "author"
    assert Author.posts._uselist is True


def test_relationship_uselist():
    """uselist flag is stored correctly."""

    class Blog(Table):
        id: int = Column(primary_key=True)
        title: str

        # One-to-many: Blog has many posts
        posts = relationship(Post, foreign_key_column="blog_id", uselist=True)

        class Settings:
            table_name = "blogs"

    assert Blog.posts._uselist is True
    assert Post.author._uselist is False  # Default is False
