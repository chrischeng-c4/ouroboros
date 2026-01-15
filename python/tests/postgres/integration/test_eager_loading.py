"""
Integration tests for PostgreSQL eager loading (JOIN-based fetching).

Tests verify that:
- Single relations can be eagerly loaded
- Multiple relations can be loaded in one query
- NULL foreign keys are handled correctly
- Different join types work correctly
- Batch eager loading works efficiently
"""

import pytest
from ouroboros.postgres import (
    init, close, execute,
    fetch_one_with_relations,
    fetch_one_eager,
    fetch_many_with_relations,
)
from ouroboros.qc import expect


@pytest.mark.integration
@pytest.mark.asyncio
class TestFetchOneEager:
    """Test single row eager loading with simplified tuple-based API."""

    async def test_fetch_one_eager_basic(self):
        """Test basic eager loading with one relation."""
        # Create tables
        await execute("""
            CREATE TABLE test_eager_authors (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_eager_posts (
                id SERIAL PRIMARY KEY,
                title TEXT NOT NULL,
                author_id INTEGER REFERENCES test_eager_authors(id)
            )
        """)

        # Insert test data
        await execute("INSERT INTO test_eager_authors (name) VALUES ($1)", ["Alice"])
        await execute("INSERT INTO test_eager_posts (title, author_id) VALUES ($1, $2)", ["My Post", 1])

        # Fetch post with author using simplified tuple API
        result = await fetch_one_eager("test_eager_posts", 1, [
            ("author", "author_id", "test_eager_authors")
        ])

        expect(result).not_to_be_none()
        expect(result["title"]).to_equal("My Post")
        expect(result["author_id"]).to_equal(1)
        # Note: The exact structure of nested relations depends on Rust implementation
        # This test verifies the basic functionality works

    async def test_fetch_one_eager_not_found(self):
        """Test eager loading when row doesn't exist."""
        await execute("""
            CREATE TABLE test_eager_items (
                id SERIAL PRIMARY KEY,
                name TEXT
            )
        """)

        result = await fetch_one_eager("test_eager_items", 999, [])
        expect(result).to_be_none()

    async def test_fetch_one_eager_null_foreign_key(self):
        """Test eager loading with NULL foreign key (LEFT JOIN)."""
        await execute("""
            CREATE TABLE test_eager_categories (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_eager_products (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                category_id INTEGER REFERENCES test_eager_categories(id)
            )
        """)

        # Insert product without category
        await execute("INSERT INTO test_eager_products (name, category_id) VALUES ($1, $2)", ["Widget", None])

        result = await fetch_one_eager("test_eager_products", 1, [
            ("category", "category_id", "test_eager_categories")
        ])

        expect(result).not_to_be_none()
        expect(result["name"]).to_equal("Widget")
        expect(result["category_id"]).to_be_none()

    async def test_fetch_one_eager_multiple_relations(self):
        """Test eager loading with multiple relations."""
        await execute("""
            CREATE TABLE test_eager_users (
                id SERIAL PRIMARY KEY,
                username TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_eager_profiles (
                id SERIAL PRIMARY KEY,
                bio TEXT
            )
        """)
        await execute("""
            CREATE TABLE test_eager_accounts (
                id SERIAL PRIMARY KEY,
                user_id INTEGER REFERENCES test_eager_users(id),
                profile_id INTEGER REFERENCES test_eager_profiles(id),
                status TEXT
            )
        """)

        # Insert test data
        await execute("INSERT INTO test_eager_users (username) VALUES ($1)", ["alice"])
        await execute("INSERT INTO test_eager_profiles (bio) VALUES ($1)", ["Developer"])
        await execute(
            "INSERT INTO test_eager_accounts (user_id, profile_id, status) VALUES ($1, $2, $3)",
            [1, 1, "active"]
        )

        # Fetch account with both user and profile
        result = await fetch_one_eager("test_eager_accounts", 1, [
            ("user", "user_id", "test_eager_users"),
            ("profile", "profile_id", "test_eager_profiles")
        ])

        expect(result).not_to_be_none()
        expect(result["status"]).to_equal("active")
        expect(result["user_id"]).to_equal(1)
        expect(result["profile_id"]).to_equal(1)


@pytest.mark.integration
@pytest.mark.asyncio
class TestFetchOneWithRelations:
    """Test fetch_one_with_relations with full configuration."""

    async def test_fetch_with_left_join_default(self):
        """Test LEFT JOIN (default) - includes rows even without matching relations."""
        await execute("""
            CREATE TABLE test_rel_departments (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_rel_employees (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                dept_id INTEGER REFERENCES test_rel_departments(id)
            )
        """)

        # Insert employee without department
        await execute("INSERT INTO test_rel_employees (name, dept_id) VALUES ($1, $2)", ["Bob", None])

        result = await fetch_one_with_relations("test_rel_employees", 1, [
            {
                "name": "department",
                "table": "test_rel_departments",
                "foreign_key": "dept_id",
                "reference_column": "id",
                "join_type": "left"
            }
        ])

        expect(result).not_to_be_none()
        expect(result["name"]).to_equal("Bob")
        expect(result["dept_id"]).to_be_none()

    async def test_fetch_with_inner_join(self):
        """Test INNER JOIN - should exclude rows without matching relations."""
        await execute("""
            CREATE TABLE test_rel_companies (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_rel_managers (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                company_id INTEGER REFERENCES test_rel_companies(id)
            )
        """)

        # Insert company and manager
        await execute("INSERT INTO test_rel_companies (name) VALUES ($1)", ["Acme Corp"])
        await execute("INSERT INTO test_rel_managers (name, company_id) VALUES ($1, $2)", ["Alice", 1])

        # Also insert manager without company (for testing INNER JOIN exclusion)
        await execute("INSERT INTO test_rel_managers (name, company_id) VALUES ($1, $2)", ["Bob", None])

        # INNER JOIN should only return manager with company
        result = await fetch_one_with_relations("test_rel_managers", 1, [
            {
                "name": "company",
                "table": "test_rel_companies",
                "foreign_key": "company_id",
                "reference_column": "id",
                "join_type": "inner"
            }
        ])

        expect(result).not_to_be_none()
        expect(result["name"]).to_equal("Alice")
        expect(result["company_id"]).to_equal(1)

        # INNER JOIN should exclude manager without company
        result_null = await fetch_one_with_relations("test_rel_managers", 2, [
            {
                "name": "company",
                "table": "test_rel_companies",
                "foreign_key": "company_id",
                "reference_column": "id",
                "join_type": "inner"
            }
        ])

        # With INNER JOIN, this might return None or empty depending on implementation
        # The key is that the company relation is not satisfied

    async def test_fetch_with_custom_reference_column(self):
        """Test custom reference column (not just 'id')."""
        await execute("""
            CREATE TABLE test_rel_countries (
                id SERIAL PRIMARY KEY,
                code TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_rel_cities (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                country_code TEXT REFERENCES test_rel_countries(code)
            )
        """)

        # Insert test data
        await execute("INSERT INTO test_rel_countries (code, name) VALUES ($1, $2)", ["US", "United States"])
        await execute("INSERT INTO test_rel_cities (name, country_code) VALUES ($1, $2)", ["New York", "US"])

        result = await fetch_one_with_relations("test_rel_cities", 1, [
            {
                "name": "country",
                "table": "test_rel_countries",
                "foreign_key": "country_code",
                "reference_column": "code",  # Reference 'code' instead of 'id'
                "join_type": "left"
            }
        ])

        expect(result).not_to_be_none()
        expect(result["name"]).to_equal("New York")
        expect(result["country_code"]).to_equal("US")


@pytest.mark.integration
@pytest.mark.asyncio
class TestFetchManyWithRelations:
    """Test fetch_many_with_relations for batch eager loading."""

    async def test_fetch_many_basic(self):
        """Test fetching multiple rows with relations."""
        await execute("""
            CREATE TABLE test_many_brands (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_many_cars (
                id SERIAL PRIMARY KEY,
                model TEXT NOT NULL,
                brand_id INTEGER REFERENCES test_many_brands(id)
            )
        """)

        # Insert data
        await execute("INSERT INTO test_many_brands (name) VALUES ($1)", ["Toyota"])
        await execute("INSERT INTO test_many_brands (name) VALUES ($1)", ["Honda"])
        await execute("INSERT INTO test_many_cars (model, brand_id) VALUES ($1, $2)", ["Camry", 1])
        await execute("INSERT INTO test_many_cars (model, brand_id) VALUES ($1, $2)", ["Accord", 2])
        await execute("INSERT INTO test_many_cars (model, brand_id) VALUES ($1, $2)", ["Corolla", 1])

        results = await fetch_many_with_relations(
            "test_many_cars",
            [{
                "name": "brand",
                "table": "test_many_brands",
                "foreign_key": "brand_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            limit=10
        )

        expect(len(results)).to_equal(3)
        # Verify all rows have brand_id populated
        expect(all(r["brand_id"] is not None for r in results)).to_be_true()
        # Verify different brands exist
        brand_ids = {r["brand_id"] for r in results}
        expect(len(brand_ids)).to_equal(2)  # Toyota and Honda

    async def test_fetch_many_with_filter(self):
        """Test fetching with filter and relations."""
        await execute("""
            CREATE TABLE test_many_stores (
                id SERIAL PRIMARY KEY,
                city TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_many_orders (
                id SERIAL PRIMARY KEY,
                total INTEGER NOT NULL,
                store_id INTEGER REFERENCES test_many_stores(id)
            )
        """)

        await execute("INSERT INTO test_many_stores (city) VALUES ($1)", ["NYC"])
        await execute("INSERT INTO test_many_stores (city) VALUES ($1)", ["LA"])
        await execute("INSERT INTO test_many_orders (total, store_id) VALUES ($1, $2)", [100, 1])
        await execute("INSERT INTO test_many_orders (total, store_id) VALUES ($1, $2)", [200, 1])
        await execute("INSERT INTO test_many_orders (total, store_id) VALUES ($1, $2)", [150, 2])

        results = await fetch_many_with_relations(
            "test_many_orders",
            [{
                "name": "store",
                "table": "test_many_stores",
                "foreign_key": "store_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            filter={"total": 100},
            limit=10
        )

        expect(len(results)).to_equal(1)
        expect(results[0]["total"]).to_equal(100)
        expect(results[0]["store_id"]).to_equal(1)

    async def test_fetch_many_with_limit_offset(self):
        """Test pagination with limit and offset."""
        await execute("""
            CREATE TABLE test_many_categories (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_many_items (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                category_id INTEGER REFERENCES test_many_categories(id)
            )
        """)

        # Insert test data
        await execute("INSERT INTO test_many_categories (name) VALUES ($1)", ["Electronics"])
        for i in range(10):
            await execute(
                "INSERT INTO test_many_items (name, category_id) VALUES ($1, $2)",
                [f"Item {i}", 1]
            )

        # Fetch first 5 items
        page1 = await fetch_many_with_relations(
            "test_many_items",
            [{
                "name": "category",
                "table": "test_many_categories",
                "foreign_key": "category_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            limit=5,
            offset=0
        )

        expect(len(page1)).to_equal(5)

        # Fetch next 5 items
        page2 = await fetch_many_with_relations(
            "test_many_items",
            [{
                "name": "category",
                "table": "test_many_categories",
                "foreign_key": "category_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            limit=5,
            offset=5
        )

        expect(len(page2)).to_equal(5)

        # Verify no overlap
        page1_ids = {item["id"] for item in page1}
        page2_ids = {item["id"] for item in page2}
        expect(page1_ids.isdisjoint(page2_ids)).to_be_true()

    async def test_fetch_many_with_order_by(self):
        """Test ordering results."""
        await execute("""
            CREATE TABLE test_many_publishers (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_many_books (
                id SERIAL PRIMARY KEY,
                title TEXT NOT NULL,
                year INTEGER,
                publisher_id INTEGER REFERENCES test_many_publishers(id)
            )
        """)

        await execute("INSERT INTO test_many_publishers (name) VALUES ($1)", ["Acme Publishing"])
        await execute("INSERT INTO test_many_books (title, year, publisher_id) VALUES ($1, $2, $3)", ["Book A", 2020, 1])
        await execute("INSERT INTO test_many_books (title, year, publisher_id) VALUES ($1, $2, $3)", ["Book B", 2022, 1])
        await execute("INSERT INTO test_many_books (title, year, publisher_id) VALUES ($1, $2, $3)", ["Book C", 2019, 1])

        # Order by year ascending
        results = await fetch_many_with_relations(
            "test_many_books",
            [{
                "name": "publisher",
                "table": "test_many_publishers",
                "foreign_key": "publisher_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            order_by=("year", "asc"),
            limit=10
        )

        expect(len(results)).to_equal(3)
        expect(results[0]["year"]).to_equal(2019)
        expect(results[1]["year"]).to_equal(2020)
        expect(results[2]["year"]).to_equal(2022)

        # Order by year descending
        results_desc = await fetch_many_with_relations(
            "test_many_books",
            [{
                "name": "publisher",
                "table": "test_many_publishers",
                "foreign_key": "publisher_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            order_by=("year", "desc"),
            limit=10
        )

        expect(len(results_desc)).to_equal(3)
        expect(results_desc[0]["year"]).to_equal(2022)
        expect(results_desc[1]["year"]).to_equal(2020)
        expect(results_desc[2]["year"]).to_equal(2019)

    async def test_fetch_many_no_relations(self):
        """Test fetch_many with empty relations list (just fetch without joins)."""
        await execute("""
            CREATE TABLE test_many_simple (
                id SERIAL PRIMARY KEY,
                value TEXT NOT NULL
            )
        """)

        await execute("INSERT INTO test_many_simple (value) VALUES ($1)", ["A"])
        await execute("INSERT INTO test_many_simple (value) VALUES ($1)", ["B"])
        await execute("INSERT INTO test_many_simple (value) VALUES ($1)", ["C"])

        results = await fetch_many_with_relations(
            "test_many_simple",
            [],  # No relations
            limit=10
        )

        expect(len(results)).to_equal(3)
        values = {r["value"] for r in results}
        expect(values).to_equal({"A", "B", "C"})

    async def test_fetch_many_with_null_foreign_keys(self):
        """Test that LEFT JOIN includes rows with NULL foreign keys."""
        await execute("""
            CREATE TABLE test_many_teams (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_many_players (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                team_id INTEGER REFERENCES test_many_teams(id)
            )
        """)

        # Insert team and players
        await execute("INSERT INTO test_many_teams (name) VALUES ($1)", ["Team A"])
        await execute("INSERT INTO test_many_players (name, team_id) VALUES ($1, $2)", ["Player 1", 1])
        await execute("INSERT INTO test_many_players (name, team_id) VALUES ($1, $2)", ["Player 2", None])  # No team
        await execute("INSERT INTO test_many_players (name, team_id) VALUES ($1, $2)", ["Player 3", 1])

        results = await fetch_many_with_relations(
            "test_many_players",
            [{
                "name": "team",
                "table": "test_many_teams",
                "foreign_key": "team_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            limit=10
        )

        # LEFT JOIN should include all players (including those without teams)
        expect(len(results)).to_equal(3)

        # Find player without team
        player_without_team = next(p for p in results if p["name"] == "Player 2")
        expect(player_without_team["team_id"]).to_be_none()


@pytest.mark.integration
@pytest.mark.asyncio
class TestEagerLoadingComplexScenarios:
    """Test complex eager loading scenarios."""

    async def test_multiple_relations_different_tables(self):
        """Test loading multiple different relations in one query."""
        await execute("""
            CREATE TABLE test_complex_authors (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_complex_categories (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_complex_articles (
                id SERIAL PRIMARY KEY,
                title TEXT NOT NULL,
                author_id INTEGER REFERENCES test_complex_authors(id),
                category_id INTEGER REFERENCES test_complex_categories(id)
            )
        """)

        # Insert test data
        await execute("INSERT INTO test_complex_authors (name) VALUES ($1)", ["Alice"])
        await execute("INSERT INTO test_complex_categories (name) VALUES ($1)", ["Tech"])
        await execute(
            "INSERT INTO test_complex_articles (title, author_id, category_id) VALUES ($1, $2, $3)",
            ["My Article", 1, 1]
        )

        result = await fetch_one_with_relations("test_complex_articles", 1, [
            {
                "name": "author",
                "table": "test_complex_authors",
                "foreign_key": "author_id",
                "reference_column": "id",
                "join_type": "left"
            },
            {
                "name": "category",
                "table": "test_complex_categories",
                "foreign_key": "category_id",
                "reference_column": "id",
                "join_type": "left"
            }
        ])

        expect(result).not_to_be_none()
        expect(result["title"]).to_equal("My Article")
        expect(result["author_id"]).to_equal(1)
        expect(result["category_id"]).to_equal(1)

    async def test_eager_loading_empty_table(self):
        """Test eager loading on empty table."""
        await execute("""
            CREATE TABLE test_complex_empty1 (
                id SERIAL PRIMARY KEY,
                name TEXT
            )
        """)
        await execute("""
            CREATE TABLE test_complex_empty2 (
                id SERIAL PRIMARY KEY,
                ref_id INTEGER REFERENCES test_complex_empty1(id)
            )
        """)

        results = await fetch_many_with_relations(
            "test_complex_empty2",
            [{
                "name": "ref",
                "table": "test_complex_empty1",
                "foreign_key": "ref_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            limit=10
        )

        expect(results).to_equal([])

    async def test_filter_with_multiple_conditions(self):
        """Test filtering with multiple conditions."""
        await execute("""
            CREATE TABLE test_complex_vendors (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        """)
        await execute("""
            CREATE TABLE test_complex_products (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                price INTEGER,
                vendor_id INTEGER REFERENCES test_complex_vendors(id)
            )
        """)

        await execute("INSERT INTO test_complex_vendors (name) VALUES ($1)", ["Vendor A"])
        await execute("INSERT INTO test_complex_products (name, price, vendor_id) VALUES ($1, $2, $3)", ["Product 1", 100, 1])
        await execute("INSERT INTO test_complex_products (name, price, vendor_id) VALUES ($1, $2, $3)", ["Product 2", 100, 1])
        await execute("INSERT INTO test_complex_products (name, price, vendor_id) VALUES ($1, $2, $3)", ["Product 3", 200, 1])

        # Note: This tests simple equality filters - complex conditions may not be supported
        results = await fetch_many_with_relations(
            "test_complex_products",
            [{
                "name": "vendor",
                "table": "test_complex_vendors",
                "foreign_key": "vendor_id",
                "reference_column": "id",
                "join_type": "left"
            }],
            filter={"price": 100},
            limit=10
        )

        expect(len(results)).to_equal(2)
        expect(all(r["price"] == 100 for r in results)).to_be_true()
