"""Base classes for PostgreSQL tests using data-bridge-test framework."""
from ouroboros.test import expect, TestMeta, TestResult, TestRunner, TestType, TestStatus


class PostgresTestBase:
    """Base class for PostgreSQL tests."""

    @classmethod
    def setup_class(cls):
        """Setup before all tests in class."""
        pass

    @classmethod
    def teardown_class(cls):
        """Teardown after all tests in class."""
        pass
