"""
Unit tests for Column cascade parameters (on_delete, on_update).

Tests foreign key cascade behavior configuration.
No database required - testing Column object creation and representation.
"""
from data_bridge.test import expect
from data_bridge.postgres import Column


class TestColumnOnDelete:
    """Test Column on_delete parameter."""

    def test_column_on_delete_cascade(self):
        """Test Column with on_delete='CASCADE'."""
        col = Column(foreign_key="users.id", on_delete="CASCADE")

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_equal("CASCADE")
        expect(col.on_update).to_be_none()

    def test_column_on_delete_restrict(self):
        """Test Column with on_delete='RESTRICT'."""
        col = Column(foreign_key="users.id", on_delete="RESTRICT")

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_equal("RESTRICT")
        expect(col.on_update).to_be_none()

    def test_column_on_delete_set_null(self):
        """Test Column with on_delete='SET NULL'."""
        col = Column(foreign_key="users.id", on_delete="SET NULL", nullable=True)

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_equal("SET NULL")
        expect(col.nullable).to_equal(True)
        expect(col.on_update).to_be_none()

    def test_column_on_delete_set_default(self):
        """Test Column with on_delete='SET DEFAULT'."""
        col = Column(foreign_key="users.id", on_delete="SET DEFAULT", default=0)

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_equal("SET DEFAULT")
        expect(col.default).to_equal(0)
        expect(col.on_update).to_be_none()

    def test_column_on_delete_no_action(self):
        """Test Column with on_delete='NO ACTION'."""
        col = Column(foreign_key="users.id", on_delete="NO ACTION")

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_equal("NO ACTION")
        expect(col.on_update).to_be_none()


class TestColumnOnUpdate:
    """Test Column on_update parameter."""

    def test_column_on_update_cascade(self):
        """Test Column with on_update='CASCADE'."""
        col = Column(foreign_key="users.id", on_update="CASCADE")

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_update).to_equal("CASCADE")
        expect(col.on_delete).to_be_none()

    def test_column_on_update_restrict(self):
        """Test Column with on_update='RESTRICT'."""
        col = Column(foreign_key="users.id", on_update="RESTRICT")

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_update).to_equal("RESTRICT")
        expect(col.on_delete).to_be_none()

    def test_column_on_update_set_null(self):
        """Test Column with on_update='SET NULL'."""
        col = Column(foreign_key="users.id", on_update="SET NULL", nullable=True)

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_update).to_equal("SET NULL")
        expect(col.nullable).to_equal(True)
        expect(col.on_delete).to_be_none()

    def test_column_on_update_set_default(self):
        """Test Column with on_update='SET DEFAULT'."""
        col = Column(foreign_key="users.id", on_update="SET DEFAULT", default=0)

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_update).to_equal("SET DEFAULT")
        expect(col.default).to_equal(0)
        expect(col.on_delete).to_be_none()

    def test_column_on_update_no_action(self):
        """Test Column with on_update='NO ACTION'."""
        col = Column(foreign_key="users.id", on_update="NO ACTION")

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_update).to_equal("NO ACTION")
        expect(col.on_delete).to_be_none()


class TestColumnBothCascadeRules:
    """Test Column with both on_delete and on_update."""

    def test_column_both_cascade_rules(self):
        """Test Column with both on_delete and on_update set."""
        col = Column(
            foreign_key="users.id",
            on_delete="CASCADE",
            on_update="CASCADE"
        )

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_equal("CASCADE")
        expect(col.on_update).to_equal("CASCADE")

    def test_column_different_cascade_rules(self):
        """Test Column with different on_delete and on_update values."""
        col = Column(
            foreign_key="users.id",
            on_delete="CASCADE",
            on_update="RESTRICT"
        )

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_equal("CASCADE")
        expect(col.on_update).to_equal("RESTRICT")

    def test_column_cascade_with_other_constraints(self):
        """Test Column with cascade rules and other constraints."""
        col = Column(
            foreign_key="users.id",
            on_delete="SET NULL",
            on_update="CASCADE",
            nullable=True,
            index=True
        )

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_equal("SET NULL")
        expect(col.on_update).to_equal("CASCADE")
        expect(col.nullable).to_equal(True)
        expect(col.index).to_equal(True)


class TestColumnReprIncludesCascade:
    """Test Column __repr__ includes cascade rules."""

    def test_column_repr_includes_cascade(self):
        """Test __repr__ includes on_delete and on_update."""
        col = Column(
            foreign_key="users.id",
            on_delete="CASCADE",
            on_update="RESTRICT"
        )
        repr_str = repr(col)

        expect("Column" in repr_str).to_be_true()
        expect("foreign_key='users.id'" in repr_str).to_be_true()
        expect("on_delete='CASCADE'" in repr_str).to_be_true()
        expect("on_update='RESTRICT'" in repr_str).to_be_true()

    def test_column_repr_only_on_delete(self):
        """Test __repr__ with only on_delete set."""
        col = Column(foreign_key="users.id", on_delete="CASCADE")
        repr_str = repr(col)

        expect("Column" in repr_str).to_be_true()
        expect("foreign_key='users.id'" in repr_str).to_be_true()
        expect("on_delete='CASCADE'" in repr_str).to_be_true()
        expect("on_update" not in repr_str).to_be_true()

    def test_column_repr_only_on_update(self):
        """Test __repr__ with only on_update set."""
        col = Column(foreign_key="users.id", on_update="CASCADE")
        repr_str = repr(col)

        expect("Column" in repr_str).to_be_true()
        expect("foreign_key='users.id'" in repr_str).to_be_true()
        expect("on_update='CASCADE'" in repr_str).to_be_true()
        expect("on_delete" not in repr_str).to_be_true()


class TestColumnDefaultNone:
    """Test Column without cascade rules defaults to None."""

    def test_column_default_none(self):
        """Test Column without on_delete/on_update defaults to None."""
        col = Column()

        expect(col.on_delete).to_be_none()
        expect(col.on_update).to_be_none()

    def test_column_with_foreign_key_no_cascade(self):
        """Test Column with foreign_key but no cascade rules."""
        col = Column(foreign_key="users.id")

        expect(col.foreign_key).to_equal("users.id")
        expect(col.on_delete).to_be_none()
        expect(col.on_update).to_be_none()

    def test_column_repr_without_cascade(self):
        """Test __repr__ without cascade rules doesn't include them."""
        col = Column(foreign_key="users.id", unique=True)
        repr_str = repr(col)

        expect("Column" in repr_str).to_be_true()
        expect("foreign_key='users.id'" in repr_str).to_be_true()
        expect("unique=True" in repr_str).to_be_true()
        expect("on_delete" not in repr_str).to_be_true()
        expect("on_update" not in repr_str).to_be_true()


class TestColumnCascadeVariations:
    """Test various cascade rule combinations and edge cases."""

    def test_column_cascade_all_actions(self):
        """Test all valid cascade action values."""
        actions = ["CASCADE", "RESTRICT", "SET NULL", "SET DEFAULT", "NO ACTION"]

        for action in actions:
            col_delete = Column(foreign_key="users.id", on_delete=action)
            col_update = Column(foreign_key="users.id", on_update=action)

            expect(col_delete.on_delete).to_equal(action)
            expect(col_update.on_update).to_equal(action)

    def test_column_cascade_case_sensitive(self):
        """Test cascade rules are stored as-is (case preserved)."""
        col = Column(foreign_key="users.id", on_delete="CaScAdE")

        expect(col.on_delete).to_equal("CaScAdE")

    def test_column_cascade_with_complex_foreign_key(self):
        """Test cascade rules with table.column foreign key syntax."""
        col = Column(
            foreign_key="organizations.org_id",
            on_delete="CASCADE",
            on_update="CASCADE"
        )

        expect(col.foreign_key).to_equal("organizations.org_id")
        expect(col.on_delete).to_equal("CASCADE")
        expect(col.on_update).to_equal("CASCADE")

    def test_column_cascade_with_simple_foreign_key(self):
        """Test cascade rules with simple table name foreign key."""
        col = Column(
            foreign_key="users",
            on_delete="RESTRICT",
            on_update="NO ACTION"
        )

        expect(col.foreign_key).to_equal("users")
        expect(col.on_delete).to_equal("RESTRICT")
        expect(col.on_update).to_equal("NO ACTION")
