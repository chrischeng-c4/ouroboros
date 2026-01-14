"""Integration tests for PostgreSQL extensions (Full-Text Search, PostGIS, Arrays)."""
import pytest
from ouroboros.test import expect
from ouroboros.postgres import (
    Table, Column, init,
    FullTextSearch, fts,
    Point, GeoQuery,
    ArrayOps,
    raiseload, selectinload,
    relationship,
)

# Test models
class Article(Table):
    id: int = Column(primary_key=True)
    title: str
    content: str

    class Settings:
        table_name = "test_articles_fts"


class Location(Table):
    id: int = Column(primary_key=True)
    name: str
    # coordinates would be PostGIS geometry type in real usage

    class Settings:
        table_name = "test_locations_postgis"


class Post(Table):
    id: int = Column(primary_key=True)
    title: str
    # tags would be text[] array type in real usage

    class Settings:
        table_name = "test_posts_arrays"


class Author(Table):
    id: int = Column(primary_key=True)
    name: str

    class Settings:
        table_name = "test_authors_raiseload"


class Book(Table):
    id: int = Column(primary_key=True)
    title: str
    author_id: int = Column(foreign_key="test_authors_raiseload.id")

    author: Author = relationship(Author, foreign_key_column="author_id")

    class Settings:
        table_name = "test_books_raiseload"


# Full-Text Search Tests
@pytest.mark.asyncio
async def test_fulltext_to_tsvector():
    """Test to_tsvector generation."""
    result = FullTextSearch.to_tsvector("content")
    assert result == "to_tsvector('english', content)"


@pytest.mark.asyncio
async def test_fulltext_to_tsvector_custom_config():
    """Test to_tsvector with custom language config."""
    result = FullTextSearch.to_tsvector("content", config="spanish")
    assert result == "to_tsvector('spanish', content)"


@pytest.mark.asyncio
async def test_fulltext_to_tsquery():
    """Test to_tsquery generation."""
    result = FullTextSearch.to_tsquery("python & database")
    assert result == "to_tsquery('english', 'python & database')"


@pytest.mark.asyncio
async def test_fulltext_to_tsquery_escapes_quotes():
    """Test to_tsquery escapes single quotes."""
    result = FullTextSearch.to_tsquery("it's working")
    assert result == "to_tsquery('english', 'it''s working')"


@pytest.mark.asyncio
async def test_fulltext_plainto_tsquery():
    """Test plainto_tsquery generation."""
    result = FullTextSearch.plainto_tsquery("python database")
    assert result == "plainto_tsquery('english', 'python database')"


@pytest.mark.asyncio
async def test_fulltext_match():
    """Test match expression generation."""
    result = FullTextSearch.match("content", "python database")
    assert "to_tsvector" in result
    assert "plainto_tsquery" in result
    assert "@@" in result
    assert result == "to_tsvector('english', content) @@ plainto_tsquery('english', 'python database')"


@pytest.mark.asyncio
async def test_fulltext_rank():
    """Test rank expression generation."""
    result = FullTextSearch.rank("content", "python database")
    assert "ts_rank" in result
    assert "to_tsvector" in result
    assert "plainto_tsquery" in result


@pytest.mark.asyncio
async def test_fulltext_alias():
    """Test fts alias works."""
    result = fts.match("content", "test")
    assert result == FullTextSearch.match("content", "test")


# PostGIS Tests
@pytest.mark.asyncio
async def test_postgis_point_creation():
    """Test Point creation."""
    point = Point(121.5, 25.0)
    assert point.lng == 121.5
    assert point.lat == 25.0
    assert point.srid == 4326


@pytest.mark.asyncio
async def test_postgis_point_custom_srid():
    """Test Point with custom SRID."""
    point = Point(121.5, 25.0, srid=3857)
    assert point.srid == 3857


@pytest.mark.asyncio
async def test_postgis_point_to_sql():
    """Test Point to_sql conversion."""
    point = Point(121.5, 25.0)
    sql = point.to_sql()
    assert sql == "ST_SetSRID(ST_MakePoint(121.5, 25.0), 4326)"


@pytest.mark.asyncio
async def test_postgis_point_from_wkt():
    """Test Point from_wkt."""
    result = Point.from_wkt("POINT(121.5 25.0)")
    assert result == "ST_GeomFromText('POINT(121.5 25.0)', 4326)"


@pytest.mark.asyncio
async def test_postgis_point_repr():
    """Test Point repr."""
    point = Point(121.5, 25.0)
    assert repr(point) == "Point(lng=121.5, lat=25.0, srid=4326)"


@pytest.mark.asyncio
async def test_postgis_distance():
    """Test ST_Distance query."""
    result = GeoQuery.distance("coordinates", "ST_MakePoint(121.5, 25.0)")
    assert result == "ST_Distance(coordinates, ST_MakePoint(121.5, 25.0))"


@pytest.mark.asyncio
async def test_postgis_dwithin():
    """Test ST_DWithin query."""
    result = GeoQuery.dwithin("coordinates", "ST_MakePoint(121.5, 25.0)", 1000)
    assert "ST_DWithin" in result
    assert "1000" in result
    assert result == "ST_DWithin(coordinates, ST_MakePoint(121.5, 25.0), 1000)"


@pytest.mark.asyncio
async def test_postgis_contains():
    """Test ST_Contains query."""
    result = GeoQuery.contains("polygon", "point")
    assert result == "ST_Contains(polygon, point)"


@pytest.mark.asyncio
async def test_postgis_within():
    """Test ST_Within query."""
    result = GeoQuery.within("point", "polygon")
    assert result == "ST_Within(point, polygon)"


@pytest.mark.asyncio
async def test_postgis_intersects():
    """Test ST_Intersects query."""
    result = GeoQuery.intersects("geom1", "geom2")
    assert result == "ST_Intersects(geom1, geom2)"


# Array Operators Tests
@pytest.mark.asyncio
async def test_array_contains():
    """Test array contains operator."""
    result = ArrayOps.contains("tags", ["python", "database"])
    assert "tags @>" in result
    assert "ARRAY[" in result
    assert result == "tags @> ARRAY['python', 'database']"


@pytest.mark.asyncio
async def test_array_contained_by():
    """Test array contained by operator."""
    result = ArrayOps.contained_by("tags", ["python", "rust", "go"])
    assert "tags <@" in result
    assert "ARRAY[" in result


@pytest.mark.asyncio
async def test_array_overlap():
    """Test array overlap operator."""
    result = ArrayOps.overlap("tags", ["python", "rust"])
    assert "tags &&" in result
    assert "ARRAY[" in result
    assert result == "tags && ARRAY['python', 'rust']"


@pytest.mark.asyncio
async def test_array_any_string():
    """Test ANY operator with string."""
    result = ArrayOps.any("tags", "python")
    assert result == "'python' = ANY(tags)"


@pytest.mark.asyncio
async def test_array_any_number():
    """Test ANY operator with number."""
    result = ArrayOps.any("scores", 100)
    assert result == "100 = ANY(scores)"


@pytest.mark.asyncio
async def test_array_length():
    """Test array_length function."""
    result = ArrayOps.length("tags")
    assert result == "array_length(tags, 1)"


@pytest.mark.asyncio
async def test_array_format_strings():
    """Test array formatting with strings."""
    result = ArrayOps._format_array(["test", "value"])
    assert result == "ARRAY['test', 'value']"


@pytest.mark.asyncio
async def test_array_format_strings_with_quotes():
    """Test array formatting with strings containing quotes."""
    result = ArrayOps._format_array(["it's", "test"])
    assert result == "ARRAY['it''s', 'test']"


@pytest.mark.asyncio
async def test_array_format_numbers():
    """Test array formatting with numbers."""
    result = ArrayOps._format_array([1, 2, 3])
    assert result == "ARRAY[1, 2, 3]"


@pytest.mark.asyncio
async def test_array_format_empty():
    """Test array formatting with empty list."""
    result = ArrayOps._format_array([])
    assert result == "ARRAY[]"


# RaiseLoad Tests (for N+1 detection)
@pytest.mark.asyncio
async def test_raiseload_raises_on_access(postgres_connection):
    """Test that raiseload raises error when relationship is accessed."""
    # Create test data
    await Author.create_table()
    await Book.create_table()

    author = await Author.insert(name="Test Author")
    book = await Book.insert(title="Test Book", author_id=author.id)

    # Load books with raiseload
    books = await Book.find().options(raiseload("author")).to_list()
    assert len(books) == 1

    # Accessing the relationship should raise
    exc_info = expect(lambda: await books[0].author).to_raise(RuntimeError)

    assert "Attempted to access unloaded relationship 'author'" in str(exc_info.value)
    assert "Use selectinload()" in str(exc_info.value)

    # Cleanup
    await Book.drop_table()
    await Author.drop_table()


@pytest.mark.asyncio
async def test_raiseload_with_selectinload_works(postgres_connection):
    """Test that selectinload prevents raiseload error."""
    # Create test data
    await Author.create_table()
    await Book.create_table()

    author = await Author.insert(name="Test Author")
    book = await Book.insert(title="Test Book", author_id=author.id)

    # Load books with selectinload (should work)
    books = await Book.find().options(selectinload("author")).to_list()
    assert len(books) == 1

    # Accessing the relationship should work
    loaded_author = await books[0].author
    assert loaded_author is not None
    assert loaded_author.name == "Test Author"

    # Cleanup
    await Book.drop_table()
    await Author.drop_table()


@pytest.mark.asyncio
async def test_raiseload_invalid_relationship():
    """Test raiseload with invalid relationship name."""
    exc_info = expect(lambda: books = await Book.find().options(raiseload("invalid_rel")).to_list()).to_raise(ValueError)

    assert "Unknown relationship: invalid_rel" in str(exc_info.value)
