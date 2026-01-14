"""
Unit tests for fetch_links parameter in Table.find() and find_one().

Run: uv run python python/tests/postgres/unit/test_fetch_links.py
"""

from ouroboros.test import TestSuite, test, expect
from ouroboros.postgres import Table
from ouroboros.postgres.relationships import relationship
from ouroboros.postgres.options import SelectInLoad


class TestFetchLinks(TestSuite):
    """Test fetch_links parameter for eager loading."""

    @test
    def test_find_without_fetch_links(self):
        """Test find() without fetch_links doesn't add options."""

        class Author(Table):
            name: str

        class Post(Table):
            title: str
            author_id: int
            author = relationship(Author, foreign_key_column="author_id")

        query = Post.find()

        # Should not have any options applied
        expect(query._options).to_equal([])

    @test
    def test_find_with_fetch_links_true(self):
        """Test find(fetch_links=True) applies selectinload for all relationships."""

        class Author(Table):
            name: str

        class Post(Table):
            title: str
            author_id: int
            author = relationship(Author, foreign_key_column="author_id")

        query = Post.find(fetch_links=True)

        # Should have selectinload applied for 'author' relationship
        expect(len(query._options)).to_equal(1)
        expect(isinstance(query._options[0], SelectInLoad)).to_equal(True)
        expect(query._options[0].relationship_name).to_equal("author")

    @test
    def test_find_with_multiple_relationships(self):
        """Test fetch_links=True with multiple relationships."""

        class Author(Table):
            name: str

        class Category(Table):
            name: str

        class Article(Table):
            title: str
            author_id: int
            category_id: int
            author = relationship(Author, foreign_key_column="author_id")
            category = relationship(Category, foreign_key_column="category_id")

        query = Article.find(fetch_links=True)

        # Should have selectinload for both relationships
        expect(len(query._options)).to_equal(2)
        relationship_names = {opt.relationship_name for opt in query._options}
        expect("author" in relationship_names).to_equal(True)
        expect("category" in relationship_names).to_equal(True)

    @test
    def test_find_with_no_relationships(self):
        """Test fetch_links=True with no relationships does nothing."""

        class SimpleTable(Table):
            name: str
            value: int

        query = SimpleTable.find(fetch_links=True)

        # Should have no options since there are no relationships
        expect(query._options).to_equal([])

    @test
    def test_find_with_filters_and_fetch_links(self):
        """Test find() combines filters with fetch_links."""

        class Author(Table):
            name: str

        class Post(Table):
            title: str
            author_id: int
            author = relationship(Author, foreign_key_column="author_id")

        query = Post.find(Post.title == "Test", fetch_links=True)

        # Should have both filter and options
        expect(len(query._filters)).to_equal(1)
        expect(len(query._options)).to_equal(1)
        expect(isinstance(query._options[0], SelectInLoad)).to_equal(True)


if __name__ == "__main__":
    import asyncio
    asyncio.run(TestFetchLinks().run())
