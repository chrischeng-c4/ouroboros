#!/usr/bin/env python3
"""
Second pass: Convert remaining pytest.raises() patterns.

Handles:
1. Property access: _ = obj.prop
2. Simple assignments: obj.prop = value
3. Method calls without await
"""

import re
from pathlib import Path
from typing import Tuple


def convert_property_access(content: str) -> Tuple[str, int]:
    """
    Convert property access patterns.

    Pattern: with pytest.raises(Exception[, match='...']):
                 _ = obj.prop
    → expect(lambda: obj.prop).to_raise(Exception)
    """
    conversions = 0
    lines = content.split('\n')
    result = []
    i = 0

    while i < len(lines):
        line = lines[i]

        # Match: with pytest.raises(ExceptionType[, match='...']):
        match = re.match(r'^(\s*)with pytest\.raises\(([^,)]+)(?:,\s*match=["\']([^"\']+)["\'])?\):\s*$', line)

        if match and i + 1 < len(lines):
            indent = match.group(1)
            exception_type = match.group(2)
            next_line = lines[i + 1]

            # Check for property access: _ = something
            prop_match = re.match(r'^\s+_\s*=\s*(.+)\s*$', next_line)
            if prop_match:
                expression = prop_match.group(1).strip()
                converted = f"{indent}expect(lambda: {expression}).to_raise({exception_type})"
                result.append(converted)
                conversions += 1
                i += 2
                continue

        result.append(line)
        i += 1

    return '\n'.join(result), conversions


def convert_simple_assignment(content: str) -> Tuple[str, int]:
    """
    Convert simple assignment patterns.

    Pattern: with pytest.raises(Exception[, match='...']):
                 obj.prop = value
    → expect(lambda: setattr(obj, 'prop', value)).to_raise(Exception)

    Or for simpler cases, keep as is with lambda.
    """
    conversions = 0
    lines = content.split('\n')
    result = []
    i = 0

    while i < len(lines):
        line = lines[i]

        # Match: with pytest.raises(ExceptionType[, match='...']):
        match = re.match(r'^(\s*)with pytest\.raises\(([^,)]+)(?:,\s*match=["\']([^"\']+)["\'])?\):\s*$', line)

        if match and i + 1 < len(lines):
            indent = match.group(1)
            exception_type = match.group(2)
            next_line = lines[i + 1]

            # Check for assignment: obj.attr = value or obj[key] = value
            assign_match = re.match(r'^\s+(.+?)\s*=\s*(.+)\s*$', next_line)
            if assign_match and '_' not in assign_match.group(1):  # Not _ =
                lhs = assign_match.group(1).strip()
                rhs = assign_match.group(2).strip()

                # Check if it's a simple attribute assignment
                if '.' in lhs and '[' not in lhs:
                    parts = lhs.rsplit('.', 1)
                    if len(parts) == 2:
                        obj = parts[0]
                        attr = parts[1]
                        converted = f"{indent}expect(lambda: setattr({obj}, '{attr}', {rhs})).to_raise({exception_type})"
                        result.append(converted)
                        conversions += 1
                        i += 2
                        continue

        result.append(line)
        i += 1

    return '\n'.join(result), conversions


def add_expect_import(content: str) -> str:
    """Add expect import if not present."""
    if 'from data_bridge.test import expect' in content:
        return content

    lines = content.split('\n')
    result = []
    added = False

    for line in lines:
        result.append(line)
        if not added and 'import pytest' in line:
            result.append('from data_bridge.test import expect')
            added = True

    return '\n'.join(result)


def convert_file(file_path: Path) -> Tuple[bool, int]:
    """Convert a single file. Returns (modified, conversions_count)."""
    try:
        content = file_path.read_text()
        original_content = content

        # Skip if no pytest.raises
        if 'pytest.raises' not in content:
            return False, 0

        total_conv = 0

        # Apply conversions
        content, prop_conv = convert_property_access(content)
        total_conv += prop_conv

        content, assign_conv = convert_simple_assignment(content)
        total_conv += assign_conv

        # Add expect import if conversions were made
        if total_conv > 0:
            content = add_expect_import(content)

        # Write back if changed
        if content != original_content:
            file_path.write_text(content)
            return True, total_conv

        return False, 0

    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False, 0


def main():
    """Main conversion routine."""
    tests_dir = Path(__file__).parent.parent / 'tests'

    # Find files that still have pytest.raises
    test_files = []
    for file_path in tests_dir.rglob('test_*.py'):
        content = file_path.read_text()
        if 'pytest.raises' in content:
            test_files.append(file_path)

    print(f"Found {len(test_files)} files with remaining pytest.raises")
    print("Starting second pass conversion...\n")

    total_files_modified = 0
    total_conversions = 0

    for file_path in test_files:
        modified, conv_count = convert_file(file_path)
        if modified:
            total_files_modified += 1
            total_conversions += conv_count
            print(f"✓ Converted {conv_count} cases in: {file_path.relative_to(tests_dir)}")

    print(f"\n{'=' * 70}")
    print("SECOND PASS SUMMARY")
    print(f"{'=' * 70}")
    print(f"Files modified:        {total_files_modified}")
    print(f"Total conversions:     {total_conversions}")
    print(f"{'=' * 70}\n")


if __name__ == '__main__':
    main()
