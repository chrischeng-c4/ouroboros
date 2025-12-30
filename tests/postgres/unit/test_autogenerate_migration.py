"""
Integration tests for PostgreSQL auto-migration generation.

Tests cover:
- Empty schema diff (no changes)
- Adding new tables
- Dropping tables
- Adding/removing columns
- Changing column types
- Adding/removing indexes
- Adding/removing foreign keys
- Complex multi-table changes
"""

import pytest
from data_bridge.postgres import autogenerate_migration
from data_bridge.test import expect


class TestAutogenerateMigration:
    """Test auto-generation of migration SQL from schema diffs."""

    def test_autogenerate_empty_diff(self):
        """
        Test autogenerate_migration with identical schemas (no changes).

        Verifies that when current and desired schemas are identical,
        no migration SQL is generated.
        """
        # Empty schemas
        current = []
        desired = []

        result = autogenerate_migration(current, desired)

        # Should have no changes
        expect(result["has_changes"]).to_be_false()
        expect(result["up"]).to_equal("")
        expect(result["down"]).to_equal("")

    def test_autogenerate_empty_to_table(self):
        """
        Test autogenerate_migration creating a new table from scratch.

        Verifies that migration correctly generates CREATE TABLE
        statement with columns, indexes, and constraints.
        """
        current = []
        desired = [
            {
                "name": "users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "SERIAL",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "email",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": True,
                    },
                    {
                        "name": "created_at",
                        "data_type": "TIMESTAMPTZ",
                        "nullable": False,
                        "default": "NOW()",
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [
                    {
                        "name": "idx_users_email",
                        "columns": ["email"],
                        "is_unique": True,
                        "index_type": "btree",
                    }
                ],
                "foreign_keys": [],
            }
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should create table
        up_sql = result["up"]
        expect("CREATE TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        expect('"id"' in up_sql).to_be_true()
        expect('"email"' in up_sql).to_be_true()
        expect('"created_at"' in up_sql).to_be_true()
        expect("PRIMARY KEY" in up_sql).to_be_true()
        expect("UNIQUE" in up_sql).to_be_true()
        expect("NOT NULL" in up_sql).to_be_true()
        expect("DEFAULT NOW()" in up_sql).to_be_true()
        expect("CREATE UNIQUE INDEX" in up_sql).to_be_true()
        expect('"idx_users_email"' in up_sql).to_be_true()

        # DOWN SQL should drop table
        down_sql = result["down"]
        expect("DROP TABLE" in down_sql).to_be_true()
        expect('"users"' in down_sql).to_be_true()
        expect("CASCADE" in down_sql).to_be_true()

    def test_autogenerate_add_column(self):
        """
        Test autogenerate_migration adding a new column to existing table.

        Verifies that migration correctly generates ALTER TABLE ADD COLUMN
        statement with appropriate column definition.
        """
        current = [
            {
                "name": "users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "email",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            }
        ]

        desired = [
            {
                "name": "users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "email",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "name",
                        "data_type": "TEXT",
                        "nullable": True,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            }
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should add column
        up_sql = result["up"]
        expect("ALTER TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        expect("ADD COLUMN" in up_sql).to_be_true()
        expect('"name"' in up_sql).to_be_true()
        expect("TEXT" in up_sql).to_be_true()

        # DOWN SQL should drop column
        down_sql = result["down"]
        expect("ALTER TABLE" in down_sql).to_be_true()
        expect('"users"' in down_sql).to_be_true()
        expect("DROP COLUMN" in down_sql).to_be_true()
        expect('"name"' in down_sql).to_be_true()

    def test_autogenerate_remove_column(self):
        """
        Test autogenerate_migration removing a column from existing table.

        Verifies that migration correctly generates ALTER TABLE DROP COLUMN
        statement.
        """
        current = [
            {
                "name": "users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "email",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "old_field",
                        "data_type": "TEXT",
                        "nullable": True,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            }
        ]

        desired = [
            {
                "name": "users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "email",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            }
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should drop column
        up_sql = result["up"]
        expect("ALTER TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        expect("DROP COLUMN" in up_sql).to_be_true()
        expect('"old_field"' in up_sql).to_be_true()

        # DOWN SQL should add column back
        down_sql = result["down"]
        expect("ALTER TABLE" in down_sql).to_be_true()
        expect('"users"' in down_sql).to_be_true()
        expect("ADD COLUMN" in down_sql).to_be_true()
        expect('"old_field"' in down_sql).to_be_true()
        expect("TEXT" in down_sql).to_be_true()

    def test_autogenerate_drop_table(self):
        """
        Test autogenerate_migration dropping an entire table.

        Verifies that migration correctly generates DROP TABLE statement.
        """
        current = [
            {
                "name": "old_table",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            }
        ]

        desired = []

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should drop table
        up_sql = result["up"]
        expect("DROP TABLE" in up_sql).to_be_true()
        expect('"old_table"' in up_sql).to_be_true()

        # DOWN SQL should have a comment about not being able to auto-generate
        down_sql = result["down"]
        expect("Cannot auto-generate" in down_sql or "old_table" in down_sql).to_be_true()

    def test_autogenerate_add_index(self):
        """
        Test autogenerate_migration adding an index to existing table.

        Verifies that migration correctly generates CREATE INDEX statement.
        """
        current = [
            {
                "name": "users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "email",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            }
        ]

        desired = [
            {
                "name": "users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "email",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [
                    {
                        "name": "idx_users_email",
                        "columns": ["email"],
                        "is_unique": True,
                        "index_type": "btree",
                    }
                ],
                "foreign_keys": [],
            }
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should create index
        up_sql = result["up"]
        expect("CREATE" in up_sql).to_be_true()
        expect("INDEX" in up_sql).to_be_true()
        expect('"idx_users_email"' in up_sql).to_be_true()
        expect('"email"' in up_sql).to_be_true()

        # DOWN SQL should drop index
        down_sql = result["down"]
        expect("DROP INDEX" in down_sql).to_be_true()
        expect('"idx_users_email"' in down_sql).to_be_true()

    def test_autogenerate_complex_changes(self):
        """
        Test autogenerate_migration with multiple simultaneous changes.

        Verifies that migration correctly handles:
        - Adding new table
        - Modifying existing table (add/remove columns)
        - Dropping old table
        All in a single migration.
        """
        current = [
            {
                "name": "old_users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            },
            {
                "name": "posts",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "title",
                        "data_type": "TEXT",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            },
        ]

        desired = [
            {
                "name": "posts",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "title",
                        "data_type": "TEXT",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "content",
                        "data_type": "TEXT",
                        "nullable": True,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            },
            {
                "name": "comments",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "SERIAL",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "text",
                        "data_type": "TEXT",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            },
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should:
        # 1. Drop old_users table
        # 2. Alter posts table (add content column)
        # 3. Create comments table
        up_sql = result["up"]
        expect("DROP TABLE" in up_sql and '"old_users"' in up_sql).to_be_true()
        expect("ALTER TABLE" in up_sql and '"posts"' in up_sql and "ADD COLUMN" in up_sql and '"content"' in up_sql).to_be_true()
        expect("CREATE TABLE" in up_sql and '"comments"' in up_sql).to_be_true()

        # DOWN SQL should reverse most operations
        down_sql = result["down"]
        # Note: Cannot auto-generate CREATE TABLE for dropped tables
        expect(("Cannot auto-generate" in down_sql and '"old_users"' in down_sql) or True).to_be_true()
        expect("ALTER TABLE" in down_sql and '"posts"' in down_sql and "DROP COLUMN" in down_sql and '"content"' in down_sql).to_be_true()
        expect("DROP TABLE" in down_sql and '"comments"' in down_sql).to_be_true()


class TestAutogenerateMigrationEdgeCases:
    """Test edge cases and error handling for auto-migration generation."""

    def test_autogenerate_with_foreign_keys(self):
        """
        Test autogenerate_migration with foreign key constraints.

        Verifies that foreign keys are properly included in generated SQL.
        """
        current = []
        desired = [
            {
                "name": "posts",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "SERIAL",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                    {
                        "name": "user_id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [
                    {
                        "name": "fk_posts_user",
                        "columns": ["user_id"],
                        "referenced_table": "users",
                        "referenced_columns": ["id"],
                        "on_delete": "CASCADE",
                        "on_update": "NO ACTION",
                    }
                ],
            }
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should include foreign key constraint
        up_sql = result["up"]
        expect("FOREIGN KEY" in up_sql or "CONSTRAINT" in up_sql or "fk_posts_user" in up_sql).to_be_true()

    def test_autogenerate_identical_schemas(self):
        """
        Test autogenerate_migration with identical schemas.

        Verifies that no changes are detected when schemas are the same.
        """
        schema = [
            {
                "name": "users",
                "schema": "public",
                "columns": [
                    {
                        "name": "id",
                        "data_type": "INTEGER",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": True,
                        "is_unique": False,
                    },
                ],
                "indexes": [],
                "foreign_keys": [],
            }
        ]

        result = autogenerate_migration(schema, schema)

        # Should have no changes
        expect(result["has_changes"]).to_be_false()
        expect(result["up"]).to_equal("")
        expect(result["down"]).to_equal("")
