"""
Integration tests for DISTINCT and DISTINCT ON functionality.

Tests query_aggregate with distinct and distinct_on parameters using a real PostgreSQL database.
"""
import pytest
from datetime import date
from data_bridge.postgres import execute, insert_one, query_aggregate


@pytest.fixture
async def employees_table():
    """Create and populate an employees table for distinct testing."""
    # Create table
    await execute(
        """
        CREATE TABLE IF NOT EXISTS employees (
            id SERIAL PRIMARY KEY,
            department VARCHAR(100) NOT NULL,
            role VARCHAR(100) NOT NULL,
            salary DECIMAL(10, 2) NOT NULL,
            hire_date DATE NOT NULL
        )
        """
    )

    # Insert test data with duplicates
    test_data = [
        # Engineering department
        {"department": "Engineering", "role": "Developer", "salary": 80000.00, "hire_date": "2023-01-15"},
        {"department": "Engineering", "role": "Developer", "salary": 85000.00, "hire_date": "2023-02-20"},
        {"department": "Engineering", "role": "Senior Developer", "salary": 120000.00, "hire_date": "2022-06-10"},
        {"department": "Engineering", "role": "Manager", "salary": 130000.00, "hire_date": "2021-03-05"},

        # Sales department
        {"department": "Sales", "role": "Sales Rep", "salary": 60000.00, "hire_date": "2023-04-12"},
        {"department": "Sales", "role": "Sales Rep", "salary": 62000.00, "hire_date": "2023-05-18"},
        {"department": "Sales", "role": "Manager", "salary": 95000.00, "hire_date": "2022-01-20"},

        # Marketing department
        {"department": "Marketing", "role": "Specialist", "salary": 70000.00, "hire_date": "2023-03-08"},
        {"department": "Marketing", "role": "Manager", "salary": 100000.00, "hire_date": "2021-09-15"},

        # Duplicate department/role combinations
        {"department": "Engineering", "role": "Developer", "salary": 82000.00, "hire_date": "2023-06-01"},
        {"department": "Sales", "role": "Sales Rep", "salary": 61000.00, "hire_date": "2023-07-10"},
    ]

    for data in test_data:
        await insert_one("employees", data)

    yield

    # Cleanup - handled by cleanup_tables fixture


@pytest.mark.asyncio
class TestDistinctSingleColumn:
    """Test DISTINCT on single column."""

    async def test_distinct_department(self, employees_table):
        """Test DISTINCT selecting unique departments."""
        results = await query_aggregate(
            "employees",
            [("count_column", "department", "dept_count")],
            group_by=["department"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc")],
            limit=None,
            distinct=True
        )

        # Should get 3 unique departments
        assert len(results) == 3
        departments = [r["department"] for r in results]
        assert set(departments) == {"Engineering", "Sales", "Marketing"}

    async def test_distinct_role(self, employees_table):
        """Test DISTINCT selecting unique roles."""
        results = await query_aggregate(
            "employees",
            [("count_column", "role", "role_count")],
            group_by=["role"],
            having=None,
            where_conditions=None,
            order_by=[("role", "asc")],
            limit=None,
            distinct=True
        )

        # Should get unique roles: Developer, Senior Developer, Manager, Sales Rep, Specialist
        assert len(results) == 5
        roles = [r["role"] for r in results]
        assert "Developer" in roles
        assert "Senior Developer" in roles
        assert "Manager" in roles
        assert "Sales Rep" in roles
        assert "Specialist" in roles

    async def test_distinct_count(self, employees_table):
        """Test COUNT with DISTINCT to count unique values."""
        results = await query_aggregate(
            "employees",
            [("count_distinct", "department", "unique_depts")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        assert results[0]["unique_depts"] == 3  # 3 unique departments


@pytest.mark.asyncio
class TestDistinctMultipleColumns:
    """Test DISTINCT on multiple columns."""

    async def test_distinct_department_role(self, employees_table):
        """Test DISTINCT on department and role combination."""
        results = await query_aggregate(
            "employees",
            [("count", None, "count")],
            group_by=["department", "role"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc"), ("role", "asc")],
            limit=None,
            distinct=True
        )

        # Unique combinations:
        # Engineering: Developer, Senior Developer, Manager (3)
        # Sales: Sales Rep, Manager (2)
        # Marketing: Specialist, Manager (2)
        # Total: 7 unique combinations
        assert len(results) == 7

    async def test_distinct_multiple_with_aggregates(self, employees_table):
        """Test DISTINCT with multiple columns and aggregate functions."""
        results = await query_aggregate(
            "employees",
            [
                ("count", None, "employee_count"),
                ("avg", "salary", "avg_salary")
            ],
            group_by=["department", "role"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc"), ("role", "asc")],
            limit=None,
            distinct=True
        )

        assert len(results) == 7

        # Find Engineering Developers
        eng_devs = [r for r in results if r["department"] == "Engineering" and r["role"] == "Developer"]
        assert len(eng_devs) == 1
        assert eng_devs[0]["employee_count"] == 3  # 3 developers
        # Average: (80000 + 85000 + 82000) / 3 = 82333.33
        assert float(eng_devs[0]["avg_salary"]) == pytest.approx(82333.33, rel=0.01)


@pytest.mark.asyncio
class TestDistinctOn:
    """Test PostgreSQL DISTINCT ON functionality."""

    async def test_distinct_on_department(self, employees_table):
        """Test DISTINCT ON to get first employee per department."""
        results = await query_aggregate(
            "employees",
            [
                ("min", "salary", "min_salary"),
                ("count", None, "count")
            ],
            group_by=["department"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc")],
            limit=None,
            distinct_on=["department"]
        )

        # Should get one result per department
        assert len(results) == 3

        # Verify departments are unique
        departments = [r["department"] for r in results]
        assert len(set(departments)) == 3

    async def test_distinct_on_with_ordering(self, employees_table):
        """Test DISTINCT ON with specific ordering to get highest salary per department."""
        results = await query_aggregate(
            "employees",
            [
                ("max", "salary", "max_salary"),
                ("count", None, "count")
            ],
            group_by=["department"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc"), ("max_salary", "desc")],
            limit=None,
            distinct_on=["department"]
        )

        assert len(results) == 3

        # Find each department and verify max salary
        dept_results = {r["department"]: r for r in results}

        # Engineering: max is 130000 (Manager)
        assert "Engineering" in dept_results

        # Sales: max is 95000 (Manager)
        assert "Sales" in dept_results

        # Marketing: max is 100000 (Manager)
        assert "Marketing" in dept_results

    async def test_distinct_on_multiple_columns(self, employees_table):
        """Test DISTINCT ON with multiple columns."""
        results = await query_aggregate(
            "employees",
            [
                ("min", "salary", "min_salary"),
                ("count", None, "count")
            ],
            group_by=["department", "role"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc"), ("role", "asc")],
            limit=None,
            distinct_on=["department", "role"]
        )

        # Should get one result per department+role combination
        assert len(results) == 7


@pytest.mark.asyncio
class TestDistinctWithAggregates:
    """Test DISTINCT combined with aggregate functions."""

    async def test_distinct_with_count_and_sum(self, employees_table):
        """Test DISTINCT with COUNT and SUM aggregates."""
        results = await query_aggregate(
            "employees",
            [
                ("count", None, "emp_count"),
                ("sum", "salary", "total_salary")
            ],
            group_by=["department"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc")],
            limit=None,
            distinct=True
        )

        assert len(results) == 3

        # Find Engineering department
        eng_result = [r for r in results if r["department"] == "Engineering"]
        assert len(eng_result) == 1
        assert eng_result[0]["emp_count"] == 5  # 5 engineering employees
        # Total: 80000 + 85000 + 120000 + 130000 + 82000 = 497000
        assert float(eng_result[0]["total_salary"]) == pytest.approx(497000.00, rel=0.01)

    async def test_distinct_with_avg_and_min_max(self, employees_table):
        """Test DISTINCT with AVG, MIN, MAX aggregates."""
        results = await query_aggregate(
            "employees",
            [
                ("avg", "salary", "avg_salary"),
                ("min", "salary", "min_salary"),
                ("max", "salary", "max_salary")
            ],
            group_by=["department"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc")],
            limit=None,
            distinct=True
        )

        assert len(results) == 3

        # Verify Sales department statistics
        sales_result = [r for r in results if r["department"] == "Sales"]
        assert len(sales_result) == 1
        # Sales: 60000, 62000, 95000, 61000
        # Avg: (60000 + 62000 + 95000 + 61000) / 4 = 69500
        assert float(sales_result[0]["avg_salary"]) == pytest.approx(69500.00, rel=0.01)
        assert float(sales_result[0]["min_salary"]) == pytest.approx(60000.00, rel=0.01)
        assert float(sales_result[0]["max_salary"]) == pytest.approx(95000.00, rel=0.01)

    async def test_distinct_count_by_role(self, employees_table):
        """Test DISTINCT counting employees per role."""
        results = await query_aggregate(
            "employees",
            [("count_distinct", "department", "dept_count")],
            group_by=["role"],
            having=None,
            where_conditions=None,
            order_by=[("role", "asc")],
            limit=None
        )

        # Roles and their departments:
        # Developer: Engineering (1 dept)
        # Senior Developer: Engineering (1 dept)
        # Manager: Engineering, Sales, Marketing (3 depts)
        # Sales Rep: Sales (1 dept)
        # Specialist: Marketing (1 dept)

        manager_result = [r for r in results if r["role"] == "Manager"]
        assert len(manager_result) == 1
        assert manager_result[0]["dept_count"] == 3  # Manager role in 3 departments


@pytest.mark.asyncio
class TestDistinctWithWhere:
    """Test DISTINCT combined with WHERE conditions."""

    async def test_distinct_with_salary_filter(self, employees_table):
        """Test DISTINCT with WHERE clause filtering by salary."""
        results = await query_aggregate(
            "employees",
            [("count", None, "high_earner_count")],
            group_by=["department"],
            having=None,
            where_conditions=[("salary", "gte", 100000)],
            order_by=[("department", "asc")],
            limit=None,
            distinct=True
        )

        # High earners (>=100000):
        # Engineering: Senior Developer (120000), Manager (130000) = 2
        # Marketing: Manager (100000) = 1
        # Sales: 0

        assert len(results) == 2  # Only Engineering and Marketing

        eng_result = [r for r in results if r["department"] == "Engineering"]
        assert len(eng_result) == 1
        assert eng_result[0]["high_earner_count"] == 2

        mkt_result = [r for r in results if r["department"] == "Marketing"]
        assert len(mkt_result) == 1
        assert mkt_result[0]["high_earner_count"] == 1

    async def test_distinct_with_role_filter(self, employees_table):
        """Test DISTINCT with WHERE filtering by role."""
        results = await query_aggregate(
            "employees",
            [
                ("count", None, "count"),
                ("avg", "salary", "avg_salary")
            ],
            group_by=["department"],
            having=None,
            where_conditions=[("role", "eq", "Manager")],
            order_by=[("department", "asc")],
            limit=None,
            distinct=True
        )

        # Only managers: Engineering, Sales, Marketing (1 each)
        assert len(results) == 3

        for result in results:
            assert result["count"] == 1  # 1 manager per department

    async def test_distinct_with_multiple_conditions(self, employees_table):
        """Test DISTINCT with multiple WHERE conditions."""
        results = await query_aggregate(
            "employees",
            [
                ("count", None, "count"),
                ("sum", "salary", "total")
            ],
            group_by=["department"],
            having=None,
            where_conditions=[
                ("salary", "gt", 70000),
                ("role", "ne", "Manager")
            ],
            order_by=[("department", "asc")],
            limit=None,
            distinct=True
        )

        # Employees with salary > 70000 AND not Manager:
        # Engineering: Developer (80000, 85000, 82000), Senior Developer (120000) = 4
        # Sales: 0 (Sales Reps all under 70000)
        # Marketing: 0 (only Manager is > 70000)

        assert len(results) == 1  # Only Engineering
        assert results[0]["department"] == "Engineering"
        assert results[0]["count"] == 4

    async def test_distinct_on_with_where(self, employees_table):
        """Test DISTINCT ON combined with WHERE clause."""
        results = await query_aggregate(
            "employees",
            [
                ("max", "salary", "max_salary"),
                ("count", None, "count")
            ],
            group_by=["department"],
            having=None,
            where_conditions=[("salary", "lt", 100000)],
            order_by=[("department", "asc")],
            limit=None,
            distinct_on=["department"]
        )

        # Employees with salary < 100000:
        # Engineering: Developer (80000, 85000, 82000) = 3
        # Sales: Sales Rep (60000, 62000, 61000) = 3
        # Marketing: Specialist (70000) = 1

        assert len(results) == 3


@pytest.mark.asyncio
class TestDistinctEdgeCases:
    """Test edge cases and special scenarios."""

    async def test_distinct_with_limit(self, employees_table):
        """Test DISTINCT combined with LIMIT."""
        results = await query_aggregate(
            "employees",
            [("count", None, "count")],
            group_by=["department"],
            having=None,
            where_conditions=None,
            order_by=[("department", "asc")],
            limit=2,
            distinct=True
        )

        # Should only return first 2 departments (alphabetically)
        assert len(results) == 2
        assert results[0]["department"] == "Engineering"
        assert results[1]["department"] == "Marketing"

    async def test_distinct_with_having(self, employees_table):
        """Test DISTINCT combined with HAVING clause."""
        results = await query_aggregate(
            "employees",
            [
                ("count", None, "emp_count"),
                ("avg", "salary", "avg_salary")
            ],
            group_by=["department"],
            having=[("count", None, "gt", 2)],
            where_conditions=None,
            order_by=[("department", "asc")],
            limit=None,
            distinct=True
        )

        # Departments with more than 2 employees:
        # Engineering: 5 employees
        # Sales: 4 employees
        # Marketing: 2 employees (excluded)

        assert len(results) == 2
        departments = [r["department"] for r in results]
        assert "Engineering" in departments
        assert "Sales" in departments
        assert "Marketing" not in departments

    async def test_distinct_empty_result(self, employees_table):
        """Test DISTINCT with WHERE clause that returns no results."""
        results = await query_aggregate(
            "employees",
            [("count", None, "count")],
            group_by=["department"],
            having=None,
            where_conditions=[("salary", "gt", 1000000)],  # No one earns this much
            order_by=None,
            limit=None,
            distinct=True
        )

        # Should return empty list
        assert len(results) == 0

    async def test_distinct_all_columns_unique(self, employees_table):
        """Test DISTINCT when all rows are already unique."""
        # Create a simple count by role (all unique anyway)
        results = await query_aggregate(
            "employees",
            [("count", None, "count")],
            group_by=["id"],
            having=None,
            where_conditions=None,
            order_by=[("id", "asc")],
            limit=5,
            distinct=True
        )

        # Should return 5 rows (limited to 5)
        assert len(results) == 5

        # Each should have count of 1
        for result in results:
            assert result["count"] == 1
