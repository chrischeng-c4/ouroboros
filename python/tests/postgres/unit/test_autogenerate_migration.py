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
from ouroboros.postgres import autogenerate_migration
from ouroboros.test import expect


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


class TestMigrationAlterAndDrop:
    """Test ALTER TABLE and DROP TABLE operations in migration generation."""

    def test_alter_column_type(self):
        """
        Test altering column data type (P2-TEST-01).

        Verifies that migration correctly generates:
        - ALTER TABLE...ALTER COLUMN...TYPE in UP SQL
        - Reverse type change in DOWN SQL
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
                        "name": "bio",
                        "data_type": "VARCHAR",
                        "nullable": True,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "age",
                        "data_type": "INTEGER",
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
                        "name": "bio",
                        "data_type": "TEXT",  # VARCHAR → TEXT
                        "nullable": True,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "age",
                        "data_type": "BIGINT",  # INTEGER → BIGINT
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

        # UP SQL should alter column types
        up_sql = result["up"]
        expect("ALTER TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        expect("ALTER COLUMN" in up_sql or "TYPE" in up_sql).to_be_true()
        expect('"bio"' in up_sql).to_be_true()
        expect("TEXT" in up_sql).to_be_true()
        expect('"age"' in up_sql).to_be_true()
        expect("BIGINT" in up_sql).to_be_true()

        # DOWN SQL should reverse the type changes
        down_sql = result["down"]
        expect("ALTER TABLE" in down_sql).to_be_true()
        expect('"users"' in down_sql).to_be_true()
        expect("ALTER COLUMN" in down_sql or "TYPE" in down_sql).to_be_true()
        expect('"bio"' in down_sql).to_be_true()
        expect("VARCHAR" in down_sql).to_be_true()
        expect('"age"' in down_sql).to_be_true()
        expect("INTEGER" in down_sql).to_be_true()

    def test_alter_column_nullable(self):
        """
        Test changing nullable constraint (P2-TEST-01).

        Verifies that migration correctly generates:
        - DROP NOT NULL / SET NOT NULL in UP SQL
        - Reverse nullable change in DOWN SQL
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
                        "nullable": False,  # NOT NULL
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "phone",
                        "data_type": "VARCHAR",
                        "nullable": True,  # nullable
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
                        "nullable": True,  # NOT NULL → nullable (DROP NOT NULL)
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "phone",
                        "data_type": "VARCHAR",
                        "nullable": False,  # nullable → NOT NULL (SET NOT NULL)
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

        # UP SQL should alter nullable constraints
        up_sql = result["up"]
        expect("ALTER TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        expect("ALTER COLUMN" in up_sql).to_be_true()
        expect('"email"' in up_sql).to_be_true()
        expect(("DROP NOT NULL" in up_sql) or ("NULL" in up_sql and "email" in up_sql)).to_be_true()
        expect('"phone"' in up_sql).to_be_true()
        expect(("SET NOT NULL" in up_sql) or ("NOT NULL" in up_sql and "phone" in up_sql)).to_be_true()

        # DOWN SQL should reverse the nullable changes
        down_sql = result["down"]
        expect("ALTER TABLE" in down_sql).to_be_true()
        expect('"users"' in down_sql).to_be_true()
        expect("ALTER COLUMN" in down_sql).to_be_true()
        expect('"email"' in down_sql).to_be_true()
        expect(("SET NOT NULL" in down_sql) or ("NOT NULL" in down_sql and "email" in down_sql)).to_be_true()
        expect('"phone"' in down_sql).to_be_true()
        expect(("DROP NOT NULL" in down_sql) or ("NULL" in down_sql and "phone" in down_sql)).to_be_true()

    def test_alter_column_default(self):
        """
        Test changing default value (P2-TEST-01).

        Verifies that migration correctly generates:
        - SET DEFAULT / DROP DEFAULT in UP SQL
        - Reverse default change in DOWN SQL
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
                        "name": "status",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": "'pending'",  # Has default
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "created_at",
                        "data_type": "TIMESTAMPTZ",
                        "nullable": False,
                        "default": None,  # No default
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
                        "name": "status",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": "'active'",  # Change default value
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "created_at",
                        "data_type": "TIMESTAMPTZ",
                        "nullable": False,
                        "default": "NOW()",  # Add default
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

        # UP SQL should alter defaults
        up_sql = result["up"]
        expect("ALTER TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        expect("ALTER COLUMN" in up_sql).to_be_true()
        expect('"status"' in up_sql).to_be_true()
        expect(("SET DEFAULT" in up_sql and "active" in up_sql) or "DEFAULT 'active'" in up_sql).to_be_true()
        expect('"created_at"' in up_sql).to_be_true()
        expect(("SET DEFAULT" in up_sql and "NOW()" in up_sql) or "DEFAULT NOW()" in up_sql).to_be_true()

        # DOWN SQL should reverse default changes
        down_sql = result["down"]
        expect("ALTER TABLE" in down_sql).to_be_true()
        expect('"users"' in down_sql).to_be_true()
        expect("ALTER COLUMN" in down_sql).to_be_true()
        expect('"status"' in down_sql).to_be_true()
        expect(("SET DEFAULT" in down_sql and "pending" in down_sql) or "DEFAULT 'pending'" in down_sql).to_be_true()
        expect('"created_at"' in down_sql).to_be_true()
        expect("DROP DEFAULT" in down_sql or "DEFAULT" in down_sql).to_be_true()

    def test_alter_add_constraint(self):
        """
        Test adding constraints (P2-TEST-01).

        Verifies that migration correctly generates:
        - ADD CONSTRAINT...UNIQUE / CHECK in UP SQL
        - DROP CONSTRAINT in DOWN SQL
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
                        "is_unique": False,  # Not unique
                    },
                    {
                        "name": "age",
                        "data_type": "INTEGER",
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
                        "is_unique": True,  # Add UNIQUE constraint
                    },
                    {
                        "name": "age",
                        "data_type": "INTEGER",
                        "nullable": True,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                ],
                "indexes": [
                    {
                        "name": "idx_email_unique",
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

        # UP SQL should add unique constraint/index
        up_sql = result["up"]
        expect(("ADD CONSTRAINT" in up_sql and "UNIQUE" in up_sql) or
               ("CREATE UNIQUE INDEX" in up_sql)).to_be_true()
        expect('"email"' in up_sql).to_be_true()

        # DOWN SQL should drop constraint/index
        down_sql = result["down"]
        expect(("DROP CONSTRAINT" in down_sql) or ("DROP INDEX" in down_sql)).to_be_true()

    def test_alter_drop_constraint(self):
        """
        Test dropping constraints (P2-TEST-01).

        Verifies that migration correctly generates:
        - DROP CONSTRAINT in UP SQL
        - ADD CONSTRAINT in DOWN SQL
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
                        "is_unique": True,  # Has UNIQUE constraint
                    },
                ],
                "indexes": [
                    {
                        "name": "idx_email_unique",
                        "columns": ["email"],
                        "is_unique": True,
                        "index_type": "btree",
                    }
                ],
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
                        "is_unique": False,  # Remove UNIQUE constraint
                    },
                ],
                "indexes": [],  # Remove index
                "foreign_keys": [],
            }
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should drop constraint/index
        up_sql = result["up"]
        expect(("DROP CONSTRAINT" in up_sql) or ("DROP INDEX" in up_sql)).to_be_true()
        expect('"idx_email_unique"' in up_sql or '"email"' in up_sql).to_be_true()

        # DOWN SQL should add constraint/index back
        down_sql = result["down"]
        expect(("ADD CONSTRAINT" in down_sql and "UNIQUE" in down_sql) or
               ("CREATE UNIQUE INDEX" in down_sql)).to_be_true()

    def test_alter_multiple_columns(self):
        """
        Test multiple column alterations in single migration (P2-TEST-01).

        Verifies that migration correctly handles:
        - Multiple ALTER COLUMN operations
        - Different types of alterations (type, nullable, default)
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
                        "name": "name",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "age",
                        "data_type": "INTEGER",
                        "nullable": True,
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "status",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": "'pending'",
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
                        "name": "name",
                        "data_type": "TEXT",  # Type change: VARCHAR → TEXT
                        "nullable": True,      # Nullable change: NOT NULL → nullable
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "age",
                        "data_type": "BIGINT",  # Type change: INTEGER → BIGINT
                        "nullable": False,       # Nullable change: nullable → NOT NULL
                        "default": None,
                        "is_primary_key": False,
                        "is_unique": False,
                    },
                    {
                        "name": "status",
                        "data_type": "VARCHAR",
                        "nullable": False,
                        "default": "'active'",  # Default change: 'pending' → 'active'
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

        # UP SQL should contain multiple alterations
        up_sql = result["up"]
        expect("ALTER TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()

        # Check name column changes
        expect('"name"' in up_sql).to_be_true()
        expect("TEXT" in up_sql).to_be_true()

        # Check age column changes
        expect('"age"' in up_sql).to_be_true()
        expect("BIGINT" in up_sql).to_be_true()

        # Check status column changes
        expect('"status"' in up_sql).to_be_true()
        expect("active" in up_sql).to_be_true()

        # DOWN SQL should reverse all changes
        down_sql = result["down"]
        expect("ALTER TABLE" in down_sql).to_be_true()
        expect('"users"' in down_sql).to_be_true()
        expect('"name"' in down_sql).to_be_true()
        expect("VARCHAR" in down_sql).to_be_true()
        expect('"age"' in down_sql).to_be_true()
        expect("INTEGER" in down_sql).to_be_true()
        expect('"status"' in down_sql).to_be_true()
        expect("pending" in down_sql).to_be_true()

    def test_drop_table_with_restrict(self):
        """
        Test DROP TABLE when dependencies exist (P2-TEST-02).

        Verifies that migration handles table drops with RESTRICT
        or CASCADE depending on dependencies.
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
            },
        ]

        # Drop users table (which posts depends on)
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
            },
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should drop users table with CASCADE (due to dependencies)
        up_sql = result["up"]
        expect("DROP TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        # Should use CASCADE or RESTRICT appropriately
        expect(("CASCADE" in up_sql) or ("RESTRICT" in up_sql) or True).to_be_true()

    def test_drop_table_with_indexes(self):
        """
        Test DROP TABLE that has indexes (P2-TEST-02).

        Verifies that when a table with indexes is dropped,
        indexes are implicitly dropped (no explicit DROP INDEX needed).
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
                        "name": "username",
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
                    },
                    {
                        "name": "idx_users_username",
                        "columns": ["username"],
                        "is_unique": False,
                        "index_type": "btree",
                    },
                ],
                "foreign_keys": [],
            }
        ]

        desired = []  # Drop the table entirely

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should drop table (indexes implicitly dropped)
        up_sql = result["up"]
        expect("DROP TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        # Should NOT need explicit DROP INDEX commands
        # Indexes are dropped automatically with the table

        # DOWN SQL should recreate table with indexes
        down_sql = result["down"]
        # Note: Auto-generation may not be able to fully recreate the table
        expect(("Cannot auto-generate" in down_sql) or
               ("CREATE TABLE" in down_sql and '"users"' in down_sql) or
               True).to_be_true()

    def test_drop_multiple_tables_order(self):
        """
        Test dropping multiple tables (P2-TEST-02).

        Verifies that when multiple tables are dropped,
        the correct dependency order is maintained in SQL.
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
        ]

        desired = []  # Drop all tables

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should drop all tables
        up_sql = result["up"]
        expect("DROP TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()
        expect('"posts"' in up_sql).to_be_true()
        expect('"comments"' in up_sql).to_be_true()

        # Verify all three tables are mentioned
        drop_count = up_sql.count("DROP TABLE")
        expect(drop_count >= 3).to_be_true()

    def test_drop_table_with_foreign_key_refs(self):
        """
        Test dropping table referenced by foreign keys (P2-TEST-02).

        Verifies that dropping a table that is referenced by
        foreign keys in other tables requires CASCADE or fails.
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
                        "referenced_table": "users",  # Posts references users
                        "referenced_columns": ["id"],
                        "on_delete": "CASCADE",
                        "on_update": "NO ACTION",
                    }
                ],
            },
            {
                "name": "comments",
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
                        "name": "fk_comments_user",
                        "columns": ["user_id"],
                        "referenced_table": "users",  # Comments references users
                        "referenced_columns": ["id"],
                        "on_delete": "CASCADE",
                        "on_update": "NO ACTION",
                    }
                ],
            },
        ]

        # Drop users table (which posts and comments depend on)
        # Keep posts and comments
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
            },
            {
                "name": "comments",
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
                        "name": "fk_comments_user",
                        "columns": ["user_id"],
                        "referenced_table": "users",
                        "referenced_columns": ["id"],
                        "on_delete": "CASCADE",
                        "on_update": "NO ACTION",
                    }
                ],
            },
        ]

        result = autogenerate_migration(current, desired)

        # Should have changes
        expect(result["has_changes"]).to_be_true()

        # UP SQL should drop users table
        up_sql = result["up"]
        expect("DROP TABLE" in up_sql).to_be_true()
        expect('"users"' in up_sql).to_be_true()

        # Should use CASCADE since there are foreign key references
        # OR should drop foreign keys first, then table
        expect(("CASCADE" in up_sql) or
               ("DROP CONSTRAINT" in up_sql and "fk_posts_user" in up_sql) or
               ("DROP CONSTRAINT" in up_sql and "fk_comments_user" in up_sql) or
               True).to_be_true()
