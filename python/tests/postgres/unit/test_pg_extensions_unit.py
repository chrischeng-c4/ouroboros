"""Unit tests for PostgreSQL extensions (no database required)."""
import pytest
from ouroboros.postgres import (
    Table, Column,
    FullTextSearch, fts,
    Point, GeoQuery,
    ArrayOps,
    raiseload, selectinload,
)


# Full-Text Search Tests
def test_fulltext_to_tsvector():
    """Test to_tsvector generation."""
    result = FullTextSearch.to_tsvector("content")
    assert result == "to_tsvector('english', content)"


def test_fulltext_to_tsvector_custom_config():
    """Test to_tsvector with custom language config."""
    result = FullTextSearch.to_tsvector("content", config="spanish")
    assert result == "to_tsvector('spanish', content)"


def test_fulltext_to_tsquery():
    """Test to_tsquery generation."""
    result = FullTextSearch.to_tsquery("python & database")
    assert result == "to_tsquery('english', 'python & database')"


def test_fulltext_to_tsquery_escapes_quotes():
    """Test to_tsquery escapes single quotes."""
    result = FullTextSearch.to_tsquery("it's working")
    assert result == "to_tsquery('english', 'it''s working')"


def test_fulltext_plainto_tsquery():
    """Test plainto_tsquery generation."""
    result = FullTextSearch.plainto_tsquery("python database")
    assert result == "plainto_tsquery('english', 'python database')"


def test_fulltext_match():
    """Test match expression generation."""
    result = FullTextSearch.match("content", "python database")
    assert "to_tsvector" in result
    assert "plainto_tsquery" in result
    assert "@@" in result
    assert result == "to_tsvector('english', content) @@ plainto_tsquery('english', 'python database')"


def test_fulltext_rank():
    """Test rank expression generation."""
    result = FullTextSearch.rank("content", "python database")
    assert "ts_rank" in result
    assert "to_tsvector" in result
    assert "plainto_tsquery" in result


def test_fulltext_alias():
    """Test fts alias works."""
    result = fts.match("content", "test")
    assert result == FullTextSearch.match("content", "test")


# PostGIS Tests
def test_postgis_point_creation():
    """Test Point creation."""
    point = Point(121.5, 25.0)
    assert point.lng == 121.5
    assert point.lat == 25.0
    assert point.srid == 4326


def test_postgis_point_custom_srid():
    """Test Point with custom SRID."""
    point = Point(121.5, 25.0, srid=3857)
    assert point.srid == 3857


def test_postgis_point_to_sql():
    """Test Point to_sql conversion."""
    point = Point(121.5, 25.0)
    sql = point.to_sql()
    assert sql == "ST_SetSRID(ST_MakePoint(121.5, 25.0), 4326)"


def test_postgis_point_from_wkt():
    """Test Point from_wkt."""
    result = Point.from_wkt("POINT(121.5 25.0)")
    assert result == "ST_GeomFromText('POINT(121.5 25.0)', 4326)"


def test_postgis_point_repr():
    """Test Point repr."""
    point = Point(121.5, 25.0)
    assert repr(point) == "Point(lng=121.5, lat=25.0, srid=4326)"


def test_postgis_distance():
    """Test ST_Distance query."""
    result = GeoQuery.distance("coordinates", "ST_MakePoint(121.5, 25.0)")
    assert result == "ST_Distance(coordinates, ST_MakePoint(121.5, 25.0))"


def test_postgis_dwithin():
    """Test ST_DWithin query."""
    result = GeoQuery.dwithin("coordinates", "ST_MakePoint(121.5, 25.0)", 1000)
    assert "ST_DWithin" in result
    assert "1000" in result
    assert result == "ST_DWithin(coordinates, ST_MakePoint(121.5, 25.0), 1000)"


def test_postgis_contains():
    """Test ST_Contains query."""
    result = GeoQuery.contains("polygon", "point")
    assert result == "ST_Contains(polygon, point)"


def test_postgis_within():
    """Test ST_Within query."""
    result = GeoQuery.within("point", "polygon")
    assert result == "ST_Within(point, polygon)"


def test_postgis_intersects():
    """Test ST_Intersects query."""
    result = GeoQuery.intersects("geom1", "geom2")
    assert result == "ST_Intersects(geom1, geom2)"


# Array Operators Tests
def test_array_contains():
    """Test array contains operator."""
    result = ArrayOps.contains("tags", ["python", "database"])
    assert "tags @>" in result
    assert "ARRAY[" in result
    assert result == "tags @> ARRAY['python', 'database']"


def test_array_contained_by():
    """Test array contained by operator."""
    result = ArrayOps.contained_by("tags", ["python", "rust", "go"])
    assert "tags <@" in result
    assert "ARRAY[" in result


def test_array_overlap():
    """Test array overlap operator."""
    result = ArrayOps.overlap("tags", ["python", "rust"])
    assert "tags &&" in result
    assert "ARRAY[" in result
    assert result == "tags && ARRAY['python', 'rust']"


def test_array_any_string():
    """Test ANY operator with string."""
    result = ArrayOps.any("tags", "python")
    assert result == "'python' = ANY(tags)"


def test_array_any_number():
    """Test ANY operator with number."""
    result = ArrayOps.any("scores", 100)
    assert result == "100 = ANY(scores)"


def test_array_length():
    """Test array_length function."""
    result = ArrayOps.length("tags")
    assert result == "array_length(tags, 1)"


def test_array_format_strings():
    """Test array formatting with strings."""
    result = ArrayOps._format_array(["test", "value"])
    assert result == "ARRAY['test', 'value']"


def test_array_format_strings_with_quotes():
    """Test array formatting with strings containing quotes."""
    result = ArrayOps._format_array(["it's", "test"])
    assert result == "ARRAY['it''s', 'test']"


def test_array_format_numbers():
    """Test array formatting with numbers."""
    result = ArrayOps._format_array([1, 2, 3])
    assert result == "ARRAY[1, 2, 3]"


def test_array_format_empty():
    """Test array formatting with empty list."""
    result = ArrayOps._format_array([])
    assert result == "ARRAY[]"


# RaiseLoad Tests
def test_raiseload_option_creation():
    """Test raiseload option can be created."""
    option = raiseload("author")
    assert option.relationship_name == "author"


def test_selectinload_still_works():
    """Test selectinload option still works."""
    option = selectinload("author")
    assert option.relationship_name == "author"


# JSONB Methods Tests
def test_jsonb_methods_exist():
    """Test that JSONB methods exist on QueryBuilder."""
    class User(Table):
        id: int = Column(primary_key=True)
        metadata: dict

        class Settings:
            table_name = "test_users"

    qb = User.find()

    # Test all JSONB methods exist
    assert hasattr(qb, "jsonb_contains")
    assert hasattr(qb, "jsonb_contained_by")
    assert hasattr(qb, "jsonb_has_key")
    assert hasattr(qb, "jsonb_has_any_key")
    assert hasattr(qb, "jsonb_has_all_keys")


def test_jsonb_contains_chainable():
    """Test jsonb_contains returns QueryBuilder for chaining."""
    class User(Table):
        id: int = Column(primary_key=True)
        metadata: dict

        class Settings:
            table_name = "test_users"

    qb = User.find()
    result = qb.jsonb_contains("metadata", {"role": "admin"})

    # Should return a QueryBuilder
    assert hasattr(result, "to_list")
    assert hasattr(result, "first")


def test_jsonb_has_key_chainable():
    """Test jsonb_has_key returns QueryBuilder for chaining."""
    class User(Table):
        id: int = Column(primary_key=True)
        metadata: dict

        class Settings:
            table_name = "test_users"

    qb = User.find()
    result = qb.jsonb_has_key("metadata", "theme")

    # Should return a QueryBuilder
    assert hasattr(result, "to_list")
    assert hasattr(result, "first")
