"""
Integration tests for JSONB operators in PostgreSQL.

Tests cover:
1. JSONB column storage and retrieval
2. JSONB containment operator @> (json_contains)
3. JSONB key exists operator ? (json_exists)
4. JSONB path extraction -> and ->>
5. JSONB array containment
6. JSONB nested object queries
7. JSONB with WHERE conditions
8. JSONB combined with aggregates
9. JSONB with ORDER BY
10. JSONB index usage (GIN index)
"""

import pytest
from ouroboros.postgres import execute, insert_one, insert_many
from ouroboros.qc import expect


@pytest.fixture
async def products_table():
    """
    Create and populate a products table with JSONB columns.

    Schema:
        - id: SERIAL PRIMARY KEY
        - name: VARCHAR(255) NOT NULL
        - attributes: JSONB (product attributes like color, size, etc.)
        - metadata: JSONB (additional metadata like tags, ratings, etc.)
    """
    # Create table with JSONB columns
    await execute("""
        CREATE TABLE IF NOT EXISTS products (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            attributes JSONB,
            metadata JSONB
        )
    """)

    # Insert test data with various JSONB structures
    test_products = [
        # Product 1: Simple object with basic types
        {
            "name": "Red T-Shirt",
            "attributes": '{"color": "red", "size": "M", "material": "cotton"}',
            "metadata": '{"tags": ["clothing", "casual"], "rating": 4.5, "inStock": true}'
        },
        # Product 2: Nested objects
        {
            "name": "Blue Jeans",
            "attributes": '{"color": "blue", "size": "L", "dimensions": {"waist": 32, "length": 34}}',
            "metadata": '{"tags": ["clothing", "denim"], "rating": 4.8, "inStock": true, "supplier": {"name": "DenimCo", "country": "USA"}}'
        },
        # Product 3: Arrays and nested structures
        {
            "name": "Running Shoes",
            "attributes": '{"color": "black", "sizes": [8, 9, 10, 11], "features": ["waterproof", "breathable"]}',
            "metadata": '{"tags": ["footwear", "sports"], "rating": 4.9, "inStock": false, "reviews": [{"user": "john", "score": 5}, {"user": "jane", "score": 4}]}'
        },
        # Product 4: Mixed types
        {
            "name": "Laptop Backpack",
            "attributes": '{"color": "gray", "capacity": "30L", "compartments": 5}',
            "metadata": '{"tags": ["bags", "tech"], "rating": 4.6, "inStock": true, "warranty": {"years": 2, "type": "limited"}}'
        },
        # Product 5: Null values
        {
            "name": "Simple Notepad",
            "attributes": '{"color": "white", "pages": 100}',
            "metadata": '{"tags": ["stationery"], "rating": null, "inStock": true}'
        },
        # Product 6: Admin role metadata (for filtering tests)
        {
            "name": "Premium Product",
            "attributes": '{"color": "gold", "limited": true}',
            "metadata": '{"tags": ["premium"], "role": "admin", "rating": 5.0, "inStock": true}'
        },
        # Product 7: Multiple tags
        {
            "name": "Winter Jacket",
            "attributes": '{"color": "navy", "size": "XL", "insulation": "down"}',
            "metadata": '{"tags": ["clothing", "winter", "outdoor"], "rating": 4.7, "inStock": true}'
        },
    ]

    for product in test_products:
        await execute(
            "INSERT INTO products (name, attributes, metadata) VALUES ($1, $2::jsonb, $3::jsonb)",
            [product["name"], product["attributes"], product["metadata"]]
        )

    yield

    # Cleanup handled by cleanup_tables fixture


@pytest.mark.asyncio
class TestJsonbStorageAndRetrieval:
    """Test basic JSONB storage and retrieval."""

    async def test_store_and_retrieve_jsonb(self, products_table):
        """Test storing and retrieving JSONB data."""
        # Query all products
        results = await execute("SELECT * FROM products ORDER BY id")

        # Verify we have all products
        expect(len(results)).to_equal(7)

        # Verify first product JSONB data
        expect(results[0]["name"]).to_equal("Red T-Shirt")
        expect(results[0]["attributes"]["color"]).to_equal("red")
        expect(results[0]["attributes"]["size"]).to_equal("M")
        expect(results[0]["metadata"]["tags"]).to_equal(["clothing", "casual"])
        expect(results[0]["metadata"]["rating"]).to_equal(4.5)
        expect(results[0]["metadata"]["inStock"]).to_be_true()

    async def test_retrieve_nested_jsonb(self, products_table):
        """Test retrieving nested JSONB structures."""
        # Query product with nested dimensions
        results = await execute(
            "SELECT * FROM products WHERE name = $1",
            ["Blue Jeans"]
        )

        expect(len(results)).to_equal(1)
        product = results[0]

        # Verify nested dimensions
        expect(product["attributes"]["dimensions"]["waist"]).to_equal(32)
        expect(product["attributes"]["dimensions"]["length"]).to_equal(34)

        # Verify nested supplier
        expect(product["metadata"]["supplier"]["name"]).to_equal("DenimCo")
        expect(product["metadata"]["supplier"]["country"]).to_equal("USA")

    async def test_retrieve_array_in_jsonb(self, products_table):
        """Test retrieving JSONB arrays."""
        # Query product with arrays
        results = await execute(
            "SELECT * FROM products WHERE name = $1",
            ["Running Shoes"]
        )

        expect(len(results)).to_equal(1)
        product = results[0]

        # Verify array of sizes
        expect(product["attributes"]["sizes"]).to_equal([8, 9, 10, 11])

        # Verify array of features
        expect(product["attributes"]["features"]).to_equal(["waterproof", "breathable"])

        # Verify array of reviews
        expect(len(product["metadata"]["reviews"])).to_equal(2)
        expect(product["metadata"]["reviews"][0]["user"]).to_equal("john")
        expect(product["metadata"]["reviews"][1]["score"]).to_equal(4)

    async def test_null_values_in_jsonb(self, products_table):
        """Test JSONB fields with null values."""
        results = await execute(
            "SELECT * FROM products WHERE name = $1",
            ["Simple Notepad"]
        )

        expect(len(results)).to_equal(1)
        product = results[0]

        # Verify null rating
        expect(product["metadata"]["rating"]).to_be_none()
        expect(product["metadata"]["inStock"]).to_be_true()


@pytest.mark.asyncio
class TestJsonbContainmentOperator:
    """Test JSONB @> containment operator (json_contains)."""

    async def test_json_contains_simple_object(self, products_table):
        """Test @> operator with simple object containment."""
        # Find products with red color
        results = await execute("""
            SELECT * FROM products
            WHERE attributes @> '{"color": "red"}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Red T-Shirt")

    async def test_json_contains_multiple_fields(self, products_table):
        """Test @> operator with multiple field containment."""
        # Find products with blue color and size L
        results = await execute("""
            SELECT * FROM products
            WHERE attributes @> '{"color": "blue", "size": "L"}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Blue Jeans")

    async def test_json_contains_nested_object(self, products_table):
        """Test @> operator with nested object containment."""
        # Find products with specific supplier
        results = await execute("""
            SELECT * FROM products
            WHERE metadata @> '{"supplier": {"country": "USA"}}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Blue Jeans")

    async def test_json_contains_array_element(self, products_table):
        """Test @> operator with array element containment."""
        # Find products with "clothing" tag
        results = await execute("""
            SELECT * FROM products
            WHERE metadata @> '{"tags": ["clothing"]}'::jsonb
            ORDER BY id
        """)

        # Should find products with "clothing" in their tags array
        expect(len(results)).to_equal(3)  # Red T-Shirt, Blue Jeans, Winter Jacket
        names = [r["name"] for r in results]
        expect("Red T-Shirt" in names).to_be_true()
        expect("Blue Jeans" in names).to_be_true()
        expect("Winter Jacket" in names).to_be_true()

    async def test_json_contains_role_admin(self, products_table):
        """Test @> operator filtering by role."""
        # Find products with admin role in metadata
        results = await execute("""
            SELECT * FROM products
            WHERE metadata @> '{"role": "admin"}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Premium Product")

    async def test_json_contains_boolean_value(self, products_table):
        """Test @> operator with boolean values."""
        # Find products that are in stock
        results = await execute("""
            SELECT * FROM products
            WHERE metadata @> '{"inStock": true}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(6)  # All except Running Shoes
        names = [r["name"] for r in results]
        expect("Running Shoes" not in names).to_be_true()

    async def test_json_contains_no_match(self, products_table):
        """Test @> operator with no matching results."""
        # Find products with non-existent attribute
        results = await execute("""
            SELECT * FROM products
            WHERE attributes @> '{"color": "purple"}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(0)


@pytest.mark.asyncio
class TestJsonbKeyExistsOperator:
    """Test JSONB ? key exists operator."""

    async def test_json_key_exists_simple(self, products_table):
        """Test ? operator for simple key existence."""
        # Find products with 'limited' attribute
        results = await execute("""
            SELECT * FROM products
            WHERE attributes ? 'limited'
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Premium Product")

    async def test_json_key_exists_all_keys(self, products_table):
        """Test ?& operator for all keys existence."""
        # Find products with both color and size
        results = await execute("""
            SELECT * FROM products
            WHERE attributes ?& array['color', 'size']
            ORDER BY id
        """)

        # Products with both color and size
        expect(len(results)).to_equal(4)  # Red T-Shirt, Blue Jeans, Running Shoes (has sizes), Winter Jacket

    async def test_json_key_exists_any_key(self, products_table):
        """Test ?| operator for any key existence."""
        # Find products with either 'compartments' or 'capacity'
        results = await execute("""
            SELECT * FROM products
            WHERE attributes ?| array['compartments', 'capacity']
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Laptop Backpack")

    async def test_json_key_not_exists(self, products_table):
        """Test for key non-existence."""
        # Find products without 'limited' attribute
        results = await execute("""
            SELECT * FROM products
            WHERE NOT (attributes ? 'limited')
            ORDER BY id
        """)

        expect(len(results)).to_equal(6)  # All except Premium Product


@pytest.mark.asyncio
class TestJsonbPathExtraction:
    """Test JSONB path extraction operators -> and ->>."""

    async def test_json_extract_text_value(self, products_table):
        """Test ->> operator for extracting text values."""
        # Extract color as text
        results = await execute("""
            SELECT name, attributes->>'color' as color
            FROM products
            WHERE attributes->>'color' = 'red'
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Red T-Shirt")
        expect(results[0]["color"]).to_equal("red")

    async def test_json_extract_nested_text(self, products_table):
        """Test ->> operator for nested path extraction."""
        # Extract supplier country
        results = await execute("""
            SELECT name, metadata->'supplier'->>'country' as country
            FROM products
            WHERE metadata->'supplier'->>'country' IS NOT NULL
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Blue Jeans")
        expect(results[0]["country"]).to_equal("USA")

    async def test_json_extract_number(self, products_table):
        """Test -> operator for extracting numeric values."""
        # Extract rating as number
        results = await execute("""
            SELECT name, (metadata->>'rating')::float as rating
            FROM products
            WHERE metadata->>'rating' IS NOT NULL
            AND (metadata->>'rating')::float > 4.7
            ORDER BY id
        """)

        expect(len(results)).to_equal(3)  # Blue Jeans (4.8), Running Shoes (4.9), Premium (5.0)
        names = [r["name"] for r in results]
        expect("Blue Jeans" in names).to_be_true()
        expect("Running Shoes" in names).to_be_true()
        expect("Premium Product" in names).to_be_true()

    async def test_json_extract_array_element(self, products_table):
        """Test -> operator for array element extraction."""
        # Extract first tag from metadata
        results = await execute("""
            SELECT name, metadata->'tags'->0 as first_tag
            FROM products
            WHERE metadata->'tags'->0 IS NOT NULL
            ORDER BY id
        """)

        expect(len(results)).to_equal(7)
        # First tag of Red T-Shirt should be "clothing"
        expect(results[0]["first_tag"]).to_equal('"clothing"')  # JSON string includes quotes

    async def test_json_path_comparison(self, products_table):
        """Test path extraction in WHERE clause."""
        # Find products with compartments > 3
        results = await execute("""
            SELECT name, (attributes->>'compartments')::int as compartments
            FROM products
            WHERE (attributes->>'compartments')::int > 3
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Laptop Backpack")
        expect(results[0]["compartments"]).to_equal(5)


@pytest.mark.asyncio
class TestJsonbArrayOperations:
    """Test JSONB array containment and operations."""

    async def test_jsonb_array_contains_element(self, products_table):
        """Test array containment for single element."""
        # Find products with size 9
        results = await execute("""
            SELECT * FROM products
            WHERE attributes->'sizes' @> '9'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Running Shoes")

    async def test_jsonb_array_contains_multiple(self, products_table):
        """Test array containment for multiple elements."""
        # Find products with both tags "clothing" and "winter"
        results = await execute("""
            SELECT * FROM products
            WHERE metadata->'tags' @> '["clothing", "winter"]'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Winter Jacket")

    async def test_jsonb_array_length(self, products_table):
        """Test getting array length."""
        # Find products with more than 2 tags
        results = await execute("""
            SELECT name, jsonb_array_length(metadata->'tags') as tag_count
            FROM products
            WHERE jsonb_array_length(metadata->'tags') > 2
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Winter Jacket")
        expect(results[0]["tag_count"]).to_equal(3)

    async def test_jsonb_array_elements(self, products_table):
        """Test expanding array elements."""
        # Get all individual tags
        results = await execute("""
            SELECT DISTINCT jsonb_array_elements_text(metadata->'tags') as tag
            FROM products
            ORDER BY tag
        """)

        # Should have unique tags
        expect(len(results) >= 6).to_be_true()  # At least: clothing, casual, denim, footwear, sports, bags, etc.
        tags = [r["tag"] for r in results]
        expect("clothing" in tags).to_be_true()
        expect("sports" in tags).to_be_true()


@pytest.mark.asyncio
class TestJsonbWithWhereConditions:
    """Test JSONB operators combined with regular WHERE conditions."""

    async def test_jsonb_and_regular_where(self, products_table):
        """Test JSONB conditions combined with regular columns."""
        # Find products with name starting with 'R' and red color
        results = await execute("""
            SELECT * FROM products
            WHERE name LIKE 'R%'
            AND attributes @> '{"color": "red"}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Red T-Shirt")

    async def test_multiple_jsonb_conditions(self, products_table):
        """Test multiple JSONB conditions."""
        # Find products with specific attributes AND metadata
        results = await execute("""
            SELECT * FROM products
            WHERE attributes @> '{"color": "blue"}'::jsonb
            AND metadata @> '{"inStock": true}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Blue Jeans")

    async def test_jsonb_or_conditions(self, products_table):
        """Test JSONB with OR conditions."""
        # Find products with red OR blue color
        results = await execute("""
            SELECT * FROM products
            WHERE attributes @> '{"color": "red"}'::jsonb
            OR attributes @> '{"color": "blue"}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(2)
        names = [r["name"] for r in results]
        expect("Red T-Shirt" in names).to_be_true()
        expect("Blue Jeans" in names).to_be_true()

    async def test_jsonb_not_condition(self, products_table):
        """Test JSONB with NOT condition."""
        # Find products NOT in stock
        results = await execute("""
            SELECT * FROM products
            WHERE NOT (metadata @> '{"inStock": true}'::jsonb)
            ORDER BY id
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Running Shoes")


@pytest.mark.asyncio
class TestJsonbWithAggregates:
    """Test JSONB combined with aggregate functions."""

    async def test_count_by_jsonb_field(self, products_table):
        """Test COUNT grouped by JSONB field."""
        # Count products by color
        results = await execute("""
            SELECT attributes->>'color' as color, COUNT(*) as count
            FROM products
            WHERE attributes->>'color' IS NOT NULL
            GROUP BY attributes->>'color'
            ORDER BY count DESC, color
        """)

        expect(len(results) >= 5).to_be_true()  # At least 5 different colors
        # Each color should have at least 1 product
        for row in results:
            expect(row["count"] >= 1).to_be_true()

    async def test_avg_with_jsonb_extraction(self, products_table):
        """Test AVG with JSONB numeric extraction."""
        # Calculate average rating
        results = await execute("""
            SELECT AVG((metadata->>'rating')::float) as avg_rating
            FROM products
            WHERE metadata->>'rating' IS NOT NULL
        """)

        expect(len(results)).to_equal(1)
        # Average of 4.5, 4.8, 4.9, 4.6, 5.0, 4.7 = 4.75
        expect(results[0]["avg_rating"] >= 4.7).to_be_true()
        expect(results[0]["avg_rating"] <= 4.8).to_be_true()

    async def test_max_min_with_jsonb(self, products_table):
        """Test MAX and MIN with JSONB fields."""
        # Get max and min ratings
        results = await execute("""
            SELECT
                MAX((metadata->>'rating')::float) as max_rating,
                MIN((metadata->>'rating')::float) as min_rating
            FROM products
            WHERE metadata->>'rating' IS NOT NULL
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["max_rating"]).to_equal(5.0)
        expect(results[0]["min_rating"]).to_equal(4.5)

    async def test_group_by_jsonb_with_having(self, products_table):
        """Test GROUP BY JSONB field with HAVING clause."""
        # Find categories with more than 1 product
        results = await execute("""
            SELECT
                metadata->'tags'->0 as primary_tag,
                COUNT(*) as count
            FROM products
            WHERE metadata->'tags'->0 IS NOT NULL
            GROUP BY metadata->'tags'->0
            HAVING COUNT(*) > 1
            ORDER BY count DESC
        """)

        # "clothing" tag should appear multiple times
        expect(len(results) >= 1).to_be_true()


@pytest.mark.asyncio
class TestJsonbWithOrderBy:
    """Test JSONB with ORDER BY clauses."""

    async def test_order_by_jsonb_text(self, products_table):
        """Test ORDER BY JSONB text field."""
        # Order by color
        results = await execute("""
            SELECT name, attributes->>'color' as color
            FROM products
            WHERE attributes->>'color' IS NOT NULL
            ORDER BY attributes->>'color' ASC
        """)

        expect(len(results)).to_equal(7)
        # First should be alphabetically first color
        colors = [r["color"] for r in results]
        # Verify ordering is correct (sorted list should match)
        sorted_colors = sorted(colors)
        expect(colors).to_equal(sorted_colors)

    async def test_order_by_jsonb_number(self, products_table):
        """Test ORDER BY JSONB numeric field."""
        # Order by rating descending
        results = await execute("""
            SELECT name, (metadata->>'rating')::float as rating
            FROM products
            WHERE metadata->>'rating' IS NOT NULL
            ORDER BY (metadata->>'rating')::float DESC
        """)

        expect(len(results)).to_equal(6)
        # First should be Premium Product with rating 5.0
        expect(results[0]["name"]).to_equal("Premium Product")
        expect(results[0]["rating"]).to_equal(5.0)

        # Verify descending order
        ratings = [r["rating"] for r in results]
        for i in range(len(ratings) - 1):
            expect(ratings[i] >= ratings[i + 1]).to_be_true()

    async def test_order_by_nested_jsonb(self, products_table):
        """Test ORDER BY nested JSONB field."""
        # Order by warranty years
        results = await execute("""
            SELECT name, (metadata->'warranty'->>'years')::int as warranty_years
            FROM products
            WHERE metadata->'warranty'->>'years' IS NOT NULL
            ORDER BY (metadata->'warranty'->>'years')::int DESC
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Laptop Backpack")

    async def test_order_by_multiple_jsonb_fields(self, products_table):
        """Test ORDER BY multiple JSONB fields."""
        # Order by inStock (desc) then rating (desc)
        results = await execute("""
            SELECT
                name,
                (metadata->>'inStock')::boolean as in_stock,
                (metadata->>'rating')::float as rating
            FROM products
            WHERE metadata->>'rating' IS NOT NULL
            ORDER BY
                (metadata->>'inStock')::boolean DESC NULLS LAST,
                (metadata->>'rating')::float DESC NULLS LAST
        """)

        # In stock products should come first
        expect(results[0]["in_stock"]).to_be_true()


@pytest.mark.asyncio
class TestJsonbIndexUsage:
    """Test JSONB GIN index creation and usage."""

    async def test_create_gin_index_on_jsonb(self, products_table):
        """Test creating GIN index on JSONB column."""
        # Create GIN index on attributes
        await execute("""
            CREATE INDEX idx_products_attributes
            ON products USING GIN (attributes)
        """)

        # Verify index exists
        indexes = await execute("""
            SELECT indexname, indexdef
            FROM pg_indexes
            WHERE tablename = 'products'
            AND indexname = 'idx_products_attributes'
        """)

        expect(len(indexes)).to_equal(1)
        expect("gin" in indexes[0]["indexdef"].lower()).to_be_true()

    async def test_gin_index_jsonb_path(self, products_table):
        """Test creating GIN index on JSONB path."""
        # Create GIN index on metadata tags
        await execute("""
            CREATE INDEX idx_products_tags
            ON products USING GIN ((metadata->'tags'))
        """)

        # Query using indexed path
        results = await execute("""
            SELECT * FROM products
            WHERE metadata->'tags' @> '["sports"]'::jsonb
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Running Shoes")

    async def test_query_performance_with_gin_index(self, products_table):
        """Test query with GIN index for containment."""
        # Create GIN index
        await execute("""
            CREATE INDEX idx_products_metadata
            ON products USING GIN (metadata)
        """)

        # Query with containment operator (should use index)
        results = await execute("""
            SELECT * FROM products
            WHERE metadata @> '{"inStock": true}'::jsonb
            ORDER BY id
        """)

        expect(len(results)).to_equal(6)

        # Verify EXPLAIN shows index usage (optional, for debugging)
        explain = await execute("""
            EXPLAIN SELECT * FROM products
            WHERE metadata @> '{"inStock": true}'::jsonb
        """)

        # Should mention the index in the plan
        plan_text = " ".join([row.get("QUERY PLAN", "") for row in explain])
        # Note: Index usage depends on table size and statistics


@pytest.mark.asyncio
class TestJsonbComplexQueries:
    """Test complex JSONB query scenarios."""

    async def test_jsonb_subquery(self, products_table):
        """Test JSONB in subquery."""
        # Find products with above-average rating
        results = await execute("""
            SELECT name, (metadata->>'rating')::float as rating
            FROM products
            WHERE (metadata->>'rating')::float > (
                SELECT AVG((metadata->>'rating')::float)
                FROM products
                WHERE metadata->>'rating' IS NOT NULL
            )
            ORDER BY rating DESC
        """)

        expect(len(results) >= 1).to_be_true()

    async def test_jsonb_case_expression(self, products_table):
        """Test JSONB with CASE expression."""
        # Categorize products by stock status
        results = await execute("""
            SELECT
                name,
                CASE
                    WHEN (metadata->>'inStock')::boolean = true THEN 'Available'
                    WHEN (metadata->>'inStock')::boolean = false THEN 'Out of Stock'
                    ELSE 'Unknown'
                END as stock_status
            FROM products
            ORDER BY name
        """)

        expect(len(results)).to_equal(7)
        # Running Shoes should be Out of Stock
        running_shoes = [r for r in results if r["name"] == "Running Shoes"]
        expect(running_shoes[0]["stock_status"]).to_equal("Out of Stock")

    async def test_jsonb_join_condition(self, products_table):
        """Test JSONB in JOIN condition."""
        # Create a temporary categories table
        await execute("""
            CREATE TEMP TABLE categories (
                category_name VARCHAR(50),
                tag_name VARCHAR(50)
            )
        """)

        await execute("""
            INSERT INTO categories (category_name, tag_name) VALUES
            ('Apparel', 'clothing'),
            ('Athletic', 'sports'),
            ('Accessories', 'bags')
        """)

        # Join products with categories using JSONB
        results = await execute("""
            SELECT DISTINCT p.name, c.category_name
            FROM products p
            JOIN categories c ON p.metadata->'tags' @> to_jsonb(c.tag_name)
            ORDER BY p.name
        """)

        expect(len(results) >= 3).to_be_true()

    async def test_jsonb_update_field(self, products_table):
        """Test updating JSONB field."""
        # Update rating for a specific product
        await execute("""
            UPDATE products
            SET metadata = jsonb_set(metadata, '{rating}', '4.95'::jsonb)
            WHERE name = 'Running Shoes'
        """)

        # Verify update
        results = await execute("""
            SELECT name, (metadata->>'rating')::float as rating
            FROM products
            WHERE name = 'Running Shoes'
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["rating"]).to_equal(4.95)

    async def test_jsonb_remove_field(self, products_table):
        """Test removing field from JSONB."""
        # Remove 'rating' field from metadata
        await execute("""
            UPDATE products
            SET metadata = metadata - 'rating'
            WHERE name = 'Simple Notepad'
        """)

        # Verify field removed
        results = await execute("""
            SELECT name, metadata
            FROM products
            WHERE name = 'Simple Notepad'
        """)

        expect(len(results)).to_equal(1)
        expect("rating" not in results[0]["metadata"]).to_be_true()

    async def test_jsonb_merge_objects(self, products_table):
        """Test merging JSONB objects."""
        # Add new field to metadata
        await execute("""
            UPDATE products
            SET metadata = metadata || '{"verified": true}'::jsonb
            WHERE name = 'Premium Product'
        """)

        # Verify merge
        results = await execute("""
            SELECT name, metadata
            FROM products
            WHERE name = 'Premium Product'
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["metadata"]["verified"]).to_be_true()
        # Original fields should still exist
        expect(results[0]["metadata"]["role"]).to_equal("admin")


@pytest.mark.asyncio
class TestJsonbEdgeCases:
    """Test JSONB edge cases and special scenarios."""

    async def test_empty_jsonb_object(self, products_table):
        """Test querying empty JSONB objects."""
        # Insert product with empty JSONB
        await execute("""
            INSERT INTO products (name, attributes, metadata)
            VALUES ('Empty Product', '{}'::jsonb, '{}'::jsonb)
        """)

        # Query for empty objects
        results = await execute("""
            SELECT name FROM products
            WHERE attributes = '{}'::jsonb
        """)

        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Empty Product")

    async def test_null_vs_missing_jsonb_field(self, products_table):
        """Test difference between NULL and missing JSONB field."""
        # Product with null rating exists (Simple Notepad)
        results_null = await execute("""
            SELECT name FROM products
            WHERE metadata->>'rating' IS NULL
        """)

        # Should include products with missing 'rating' field too
        expect(len(results_null) >= 1).to_be_true()

    async def test_jsonb_type_checking(self, products_table):
        """Test JSONB type checking functions."""
        # Check type of rating field
        results = await execute("""
            SELECT
                name,
                jsonb_typeof(metadata->'rating') as rating_type
            FROM products
            WHERE metadata->'rating' IS NOT NULL
            ORDER BY name
        """)

        # All ratings should be 'number' type
        for row in results:
            if row["rating_type"] != "null":
                expect(row["rating_type"]).to_equal("number")

    async def test_large_jsonb_document(self, products_table):
        """Test storing and querying large JSONB document."""
        # Create a large JSONB with many fields
        large_json = '{"field1": "value1", "field2": "value2", "field3": "value3", '
        for i in range(4, 51):
            large_json += f'"field{i}": "value{i}", '
        large_json += '"field51": "value51"}'

        await execute("""
            INSERT INTO products (name, attributes, metadata)
            VALUES ('Large JSON Product', $1::jsonb, '{}'::jsonb)
        """, [large_json])

        # Query the large document
        results = await execute("""
            SELECT name, attributes
            FROM products
            WHERE name = 'Large JSON Product'
        """)

        expect(len(results)).to_equal(1)
        expect(len(results[0]["attributes"])).to_equal(51)
        expect(results[0]["attributes"]["field50"]).to_equal("value50")
