#!/usr/bin/env python3
"""
pytest to data-bridge-test Migration Tool

Automatically transforms pytest test files to use the data-bridge-test framework.
Handles imports, decorators, assertions, and class structures.

Usage:
    python tools/migrate_to_data_bridge_test.py tests/unit/test_example.py
    python tools/migrate_to_data_bridge_test.py tests/unit/*.py --dry-run
    python tools/migrate_to_data_bridge_test.py tests/ --recursive
"""

import argparse
import ast
import os
import sys
from pathlib import Path
from typing import List, Optional, Tuple


class PytestToDataBridgeTransformer(ast.NodeTransformer):
    """
    AST transformer that converts pytest code to data-bridge-test.

    Handles:
    - Import statements
    - Decorators (fixtures, parametrize, asyncio)
    - Assert statements to expect() calls
    - Class inheritance (TestSuite)
    - pytest.raises context managers
    """

    def __init__(self):
        self.warnings: List[str] = []
        self.has_pytest_import = False
        self.has_test_decorator = False
        self.modified = False

    def visit_Import(self, node: ast.Import) -> Optional[ast.AST]:
        """Transform 'import pytest' statements."""
        new_names = []
        for alias in node.names:
            if alias.name == 'pytest':
                self.has_pytest_import = True
                self.modified = True
                # Remove pytest import (will be replaced)
                continue
            new_names.append(alias)

        if not new_names:
            # If all imports were pytest, return the data-bridge-test import
            return self._create_data_bridge_import()

        node.names = new_names
        return node

    def visit_ImportFrom(self, node: ast.ImportFrom) -> Optional[ast.AST]:
        """Transform 'from pytest import ...' statements."""
        if node.module and 'pytest' in node.module:
            self.has_pytest_import = True
            self.modified = True
            # Remove pytest imports (will be replaced)
            return None
        return node

    def visit_Module(self, node: ast.Module) -> ast.Module:
        """Transform module, ensuring data-bridge-test import is at top."""
        # Visit all child nodes first
        node = self.generic_visit(node)

        # If we found pytest imports, add data-bridge-test import
        if self.has_pytest_import:
            import_node = self._create_data_bridge_import()
            # Insert after docstring if exists
            insert_pos = 0
            if (node.body and isinstance(node.body[0], ast.Expr) and
                isinstance(node.body[0].value, ast.Constant) and
                isinstance(node.body[0].value.value, str)):
                insert_pos = 1

            # Check if data-bridge-test import already exists
            has_db_import = False
            for stmt in node.body:
                if isinstance(stmt, ast.ImportFrom) and stmt.module == 'data_bridge.test':
                    has_db_import = True
                    break

            if not has_db_import:
                node.body.insert(insert_pos, import_node)

        return node

    def visit_ClassDef(self, node: ast.ClassDef) -> ast.ClassDef:
        """Transform class definition to add TestSuite base if needed."""
        # Check if this is a test class (starts with Test)
        if node.name.startswith('Test'):
            # Check if it already has bases
            has_testsuite = False
            for base in node.bases:
                if isinstance(base, ast.Name) and base.id == 'TestSuite':
                    has_testsuite = True
                    break

            # Add TestSuite base if not present and not already has bases
            if not has_testsuite:
                self.modified = True
                node.bases.append(ast.Name(id='TestSuite', ctx=ast.Load()))

        # Visit child nodes
        return self.generic_visit(node)

    def visit_FunctionDef(self, node: ast.FunctionDef) -> ast.FunctionDef:
        """Transform function decorators and add @test if needed."""
        new_decorators = []
        is_fixture = False
        is_test = False

        for dec in node.decorator_list:
            converted = self._convert_decorator(dec)
            if converted is None:
                self.modified = True
                continue
            elif converted != dec:
                self.modified = True

            # Check decorator type
            if isinstance(converted, ast.Call):
                if isinstance(converted.func, ast.Name):
                    if converted.func.id == 'fixture':
                        is_fixture = True
                    elif converted.func.id == 'test':
                        is_test = True
            elif isinstance(converted, ast.Name):
                if converted.id == 'test':
                    is_test = True

            new_decorators.append(converted)

        node.decorator_list = new_decorators

        # Add @test decorator if this is a test function without decorator
        if node.name.startswith('test_') and not is_test and not is_fixture:
            self.modified = True
            self.has_test_decorator = True
            test_decorator = ast.Name(id='test', ctx=ast.Load())
            node.decorator_list.insert(0, test_decorator)

        # Visit function body
        return self.generic_visit(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> ast.AsyncFunctionDef:
        """Transform async function decorators."""
        new_decorators = []
        is_fixture = False
        is_test = False

        for dec in node.decorator_list:
            converted = self._convert_decorator(dec)
            if converted is None:
                self.modified = True
                continue
            elif converted != dec:
                self.modified = True

            # Check decorator type
            if isinstance(converted, ast.Call):
                if isinstance(converted.func, ast.Name):
                    if converted.func.id == 'fixture':
                        is_fixture = True
                    elif converted.func.id == 'test':
                        is_test = True
            elif isinstance(converted, ast.Name):
                if converted.id == 'test':
                    is_test = True

            new_decorators.append(converted)

        node.decorator_list = new_decorators

        # Add @test decorator if this is a test function without decorator
        if node.name.startswith('test_') and not is_test and not is_fixture:
            self.modified = True
            self.has_test_decorator = True
            test_decorator = ast.Name(id='test', ctx=ast.Load())
            node.decorator_list.insert(0, test_decorator)

        # Visit function body
        return self.generic_visit(node)

    def visit_Assert(self, node: ast.Assert) -> ast.Expr:
        """Convert assert statements to expect() calls."""
        self.modified = True
        return self._convert_assert_to_expect(node)

    def visit_With(self, node: ast.With) -> ast.AST:
        """Handle pytest.raises context managers."""
        # Check if this is pytest.raises
        for item in node.items:
            if self._is_pytest_raises(item.context_expr):
                self.modified = True
                return self._convert_pytest_raises(node, item)

        return self.generic_visit(node)

    def _create_data_bridge_import(self) -> ast.ImportFrom:
        """Create the data-bridge-test import statement."""
        return ast.ImportFrom(
            module='data_bridge.test',
            names=[
                ast.alias(name='TestSuite', asname=None),
                ast.alias(name='test', asname=None),
                ast.alias(name='fixture', asname=None),
                ast.alias(name='expect', asname=None),
                ast.alias(name='parametrize', asname=None),
            ],
            level=0
        )

    def _convert_decorator(self, dec: ast.AST) -> Optional[ast.AST]:
        """Convert pytest decorators to data-bridge-test equivalents."""
        # pytest.fixture -> fixture
        if self._is_pytest_decorator(dec, 'fixture'):
            return self._convert_fixture_decorator(dec)

        # pytest.mark.asyncio -> Remove (implicit async support)
        elif self._is_pytest_mark(dec, 'asyncio'):
            return None  # Remove decorator

        # pytest.mark.parametrize -> parametrize
        elif self._is_pytest_mark(dec, 'parametrize'):
            return self._convert_parametrize_decorator(dec)

        # Other pytest.mark.* decorators
        elif self._is_pytest_mark(dec, None):
            # Keep as-is for now, but warn
            self.warnings.append(f"Unsupported pytest.mark decorator: {ast.unparse(dec)}")
            return dec

        return dec

    def _convert_fixture_decorator(self, dec: ast.AST) -> ast.AST:
        """Convert pytest.fixture to data-bridge-test fixture."""
        if isinstance(dec, ast.Call):
            # pytest.fixture(scope="class") -> fixture(scope="class")
            func = ast.Name(id='fixture', ctx=ast.Load())
            return ast.Call(
                func=func,
                args=dec.args,
                keywords=dec.keywords
            )
        else:
            # pytest.fixture -> fixture
            return ast.Name(id='fixture', ctx=ast.Load())

    def _convert_parametrize_decorator(self, dec: ast.AST) -> ast.AST:
        """Convert pytest.mark.parametrize to data-bridge-test parametrize."""
        if isinstance(dec, ast.Call) and len(dec.args) >= 2:
            # pytest.mark.parametrize("x", [1,2,3]) -> parametrize("x", [1,2,3])
            func = ast.Name(id='parametrize', ctx=ast.Load())
            return ast.Call(
                func=func,
                args=dec.args,
                keywords=dec.keywords
            )

        self.warnings.append(f"Invalid parametrize decorator: {ast.unparse(dec)}")
        return dec

    def _convert_assert_to_expect(self, node: ast.Assert) -> ast.Expr:
        """Convert assert statement to expect() call."""
        test = node.test

        # Handle comparison operations
        if isinstance(test, ast.Compare):
            return self._convert_compare_to_expect(test)

        # Handle unary operations (assert not x)
        elif isinstance(test, ast.UnaryOp) and isinstance(test.op, ast.Not):
            # assert not x -> expect(x).to_be_falsy()
            return self._make_expect_call(test.operand, 'to_be_falsy', [])

        # Handle boolean expressions (assert x)
        else:
            # assert x -> expect(x).to_be_truthy()
            return self._make_expect_call(test, 'to_be_truthy', [])

    def _convert_compare_to_expect(self, comp: ast.Compare) -> ast.Expr:
        """Convert comparison to expect() call."""
        left = comp.left
        ops = comp.ops
        comparators = comp.comparators

        if len(ops) != 1 or len(comparators) != 1:
            # Complex comparison, keep as-is and warn
            self.warnings.append(f"Complex comparison not fully supported: {ast.unparse(comp)}")
            return ast.Expr(value=comp)

        op = ops[0]
        right = comparators[0]

        # Map comparison operators to expect methods
        if isinstance(op, ast.Eq):
            return self._make_expect_call(left, 'to_equal', [right])
        elif isinstance(op, ast.NotEq):
            return self._make_expect_call(left, 'to_not_equal', [right])
        elif isinstance(op, ast.Gt):
            return self._make_expect_call(left, 'to_be_greater_than', [right])
        elif isinstance(op, ast.Lt):
            return self._make_expect_call(left, 'to_be_less_than', [right])
        elif isinstance(op, ast.GtE):
            return self._make_expect_call(left, 'to_be_greater_than_or_equal', [right])
        elif isinstance(op, ast.LtE):
            return self._make_expect_call(left, 'to_be_less_than_or_equal', [right])
        elif isinstance(op, ast.In):
            # x in y -> expect(y).to_contain(x)
            return self._make_expect_call(right, 'to_contain', [left])
        elif isinstance(op, ast.NotIn):
            # x not in y -> expect(y).to_not_contain(x)
            return self._make_expect_call(right, 'to_not_contain', [left])
        elif isinstance(op, ast.Is):
            if isinstance(right, ast.Constant) and right.value is None:
                # x is None -> expect(x).to_be_none()
                return self._make_expect_call(left, 'to_be_none', [])
            else:
                # x is y -> expect(x).to_equal(y) (with warning)
                self.warnings.append(f"'is' comparison converted to to_equal: {ast.unparse(comp)}")
                return self._make_expect_call(left, 'to_equal', [right])
        elif isinstance(op, ast.IsNot):
            if isinstance(right, ast.Constant) and right.value is None:
                # x is not None -> expect(x).to_not_be_none()
                return self._make_expect_call(left, 'to_not_be_none', [])
            else:
                # x is not y -> expect(x).to_not_equal(y) (with warning)
                self.warnings.append(f"'is not' comparison converted to to_not_equal: {ast.unparse(comp)}")
                return self._make_expect_call(left, 'to_not_equal', [right])
        else:
            # Unsupported operator
            self.warnings.append(f"Unsupported comparison operator: {ast.unparse(comp)}")
            return ast.Expr(value=comp)

    def _make_expect_call(self, value: ast.AST, method: str, args: List[ast.AST]) -> ast.Expr:
        """Create an expect() method call expression."""
        # expect(value)
        expect_call = ast.Call(
            func=ast.Name(id='expect', ctx=ast.Load()),
            args=[value],
            keywords=[]
        )

        # expect(value).method(args)
        method_call = ast.Call(
            func=ast.Attribute(
                value=expect_call,
                attr=method,
                ctx=ast.Load()
            ),
            args=args,
            keywords=[]
        )

        return ast.Expr(value=method_call)

    def _convert_pytest_raises(self, with_node: ast.With, item: ast.withitem) -> ast.AST:
        """Convert pytest.raises to expect().to_raise()."""
        # Extract exception type from pytest.raises(ExceptionType)
        context_expr = item.context_expr
        exception_type = None

        if isinstance(context_expr, ast.Call):
            if len(context_expr.args) > 0:
                exception_type = context_expr.args[0]

        if exception_type is None:
            self.warnings.append(f"Could not extract exception type from: {ast.unparse(context_expr)}")
            return with_node

        # For now, keep pytest.raises as-is but add a warning
        # Converting to lambda with raise statements is complex and error-prone
        # Manual migration is recommended for pytest.raises
        self.warnings.append(
            f"pytest.raises() requires manual migration: {ast.unparse(with_node)[:80]}..."
        )

        # Return the original node unchanged
        return with_node

    def _is_pytest_decorator(self, node: ast.AST, name: str) -> bool:
        """Check if decorator is pytest.decorator_name."""
        if isinstance(node, ast.Attribute):
            return (isinstance(node.value, ast.Name) and
                   node.value.id == 'pytest' and
                   node.attr == name)
        elif isinstance(node, ast.Call):
            return self._is_pytest_decorator(node.func, name)
        return False

    def _is_pytest_mark(self, node: ast.AST, mark_name: Optional[str]) -> bool:
        """Check if decorator is pytest.mark.mark_name (or any pytest.mark.* if name is None)."""
        if isinstance(node, ast.Attribute):
            if isinstance(node.value, ast.Attribute):
                # pytest.mark.something
                if (isinstance(node.value.value, ast.Name) and
                    node.value.value.id == 'pytest' and
                    node.value.attr == 'mark'):
                    if mark_name is None:
                        return True
                    return node.attr == mark_name
        elif isinstance(node, ast.Call):
            return self._is_pytest_mark(node.func, mark_name)
        return False

    def _is_pytest_raises(self, node: ast.AST) -> bool:
        """Check if expression is pytest.raises(...)."""
        if isinstance(node, ast.Call):
            func = node.func
            if isinstance(func, ast.Attribute):
                return (isinstance(func.value, ast.Name) and
                       func.value.id == 'pytest' and
                       func.attr == 'raises')
        return False


def transform_file(file_path: Path, dry_run: bool = False) -> Tuple[bool, List[str]]:
    """
    Transform a single pytest file to data-bridge-test.

    Args:
        file_path: Path to the pytest file
        dry_run: If True, don't write changes to disk

    Returns:
        Tuple of (success, warnings)
    """
    try:
        # Read original file
        with open(file_path, 'r', encoding='utf-8') as f:
            source = f.read()

        # Parse AST
        try:
            tree = ast.parse(source, filename=str(file_path))
        except SyntaxError as e:
            return False, [f"Syntax error in {file_path}: {e}"]

        # Transform
        transformer = PytestToDataBridgeTransformer()
        new_tree = transformer.visit(tree)

        # Only write if changes were made
        if not transformer.modified:
            return True, [f"No changes needed for {file_path}"]

        # Fix missing locations in AST
        ast.fix_missing_locations(new_tree)

        # Generate new code
        try:
            new_source = ast.unparse(new_tree)
        except Exception as e:
            return False, [f"Failed to generate code for {file_path}: {e}"]

        # Write to file (if not dry-run)
        if not dry_run:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_source)

            print(f"✓ Migrated: {file_path}")
        else:
            print(f"[DRY-RUN] Would migrate: {file_path}")

        return True, transformer.warnings

    except Exception as e:
        return False, [f"Error processing {file_path}: {e}"]


def find_test_files(path: Path, recursive: bool = False) -> List[Path]:
    """
    Find all test files in the given path.

    Args:
        path: Directory or file path
        recursive: If True, search recursively

    Returns:
        List of test file paths
    """
    if path.is_file():
        if path.name.startswith('test_') and path.suffix == '.py':
            return [path]
        return []

    if recursive:
        return list(path.rglob('test_*.py'))
    else:
        return list(path.glob('test_*.py'))


def main():
    parser = argparse.ArgumentParser(
        description='Migrate pytest tests to data-bridge-test framework',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Migrate single file
  python tools/migrate_to_data_bridge_test.py tests/unit/test_example.py

  # Dry-run on multiple files
  python tools/migrate_to_data_bridge_test.py tests/unit/*.py --dry-run

  # Recursively migrate all tests
  python tools/migrate_to_data_bridge_test.py tests/ --recursive

  # Save warnings to file
  python tools/migrate_to_data_bridge_test.py tests/ --recursive --warnings warnings.txt
        """
    )

    parser.add_argument(
        'paths',
        nargs='+',
        type=Path,
        help='Test files or directories to migrate'
    )
    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Show what would be changed without modifying files'
    )
    parser.add_argument(
        '--recursive',
        action='store_true',
        help='Recursively search for test files in directories'
    )
    parser.add_argument(
        '--warnings',
        type=Path,
        help='Save warnings to file'
    )

    args = parser.parse_args()

    # Collect all test files
    all_files = []
    for path in args.paths:
        if not path.exists():
            print(f"Error: Path does not exist: {path}", file=sys.stderr)
            sys.exit(1)

        files = find_test_files(path, args.recursive)
        all_files.extend(files)

    if not all_files:
        print("No test files found", file=sys.stderr)
        sys.exit(1)

    print(f"Found {len(all_files)} test file(s)")
    print()

    # Transform each file
    all_warnings = []
    success_count = 0
    failed_count = 0

    for file_path in all_files:
        success, warnings = transform_file(file_path, args.dry_run)

        if success:
            success_count += 1
        else:
            failed_count += 1

        if warnings:
            all_warnings.extend([f"{file_path}: {w}" for w in warnings])

    print()
    print("=" * 60)
    print(f"Migration complete:")
    print(f"  Success: {success_count}")
    print(f"  Failed: {failed_count}")
    print(f"  Warnings: {len(all_warnings)}")

    if all_warnings:
        print()
        print("Warnings:")
        for warning in all_warnings[:20]:  # Show first 20 warnings
            print(f"  - {warning}")

        if len(all_warnings) > 20:
            print(f"  ... and {len(all_warnings) - 20} more")

        # Save warnings to file if requested
        if args.warnings:
            with open(args.warnings, 'w', encoding='utf-8') as f:
                for warning in all_warnings:
                    f.write(f"{warning}\n")
            print(f"\nWarnings saved to: {args.warnings}")

    sys.exit(0 if failed_count == 0 else 1)


if __name__ == '__main__':
    main()
