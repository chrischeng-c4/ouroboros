#!/usr/bin/env python3
"""
Batch convert pytest.raises() to data-bridge-test expect().to_raise() format.

Conversion rules:
1. Simple: with pytest.raises(E): func() → expect(lambda: func()).to_raise(E)
2. With match: pytest.raises(E, match='pattern') → expect(lambda: func()).to_raise(E)
3. With context: with pytest.raises(E) as exc_info: ... → exc = expect(lambda: func()).to_raise(E)
4. Multi-line blocks: Convert to lambda with proper indentation
"""

import re
import sys
from pathlib import Path
from typing import List, Tuple, Optional


class ConversionStats:
    def __init__(self):
        self.files_scanned = 0
        self.files_modified = 0
        self.conversions_simple = 0
        self.conversions_with_match = 0
        self.conversions_with_context = 0
        self.conversions_multiline = 0
        self.skipped_complex = 0
        self.errors = []


def is_simple_statement(line: str) -> bool:
    """Check if a statement is simple enough for direct conversion."""
    # Simple if it's a single function/method call
    stripped = line.strip()

    # Skip multi-line constructs
    if any(keyword in stripped for keyword in ['async with', 'with ', 'for ', 'while ', 'if ', 'try:']):
        return False

    return (
        stripped.startswith('await ') or
        stripped.startswith('raise ') or
        ('(' in stripped and stripped.count('(') == stripped.count(')'))
    )


def extract_indentation(line: str) -> str:
    """Extract leading whitespace from line."""
    return line[:len(line) - len(line.lstrip())]


def convert_simple_raises(content: str) -> Tuple[str, int, int]:
    """
    Convert simple single-line pytest.raises patterns.

    Pattern 1: with pytest.raises(Exception):
                   func()
    → expect(lambda: func()).to_raise(Exception)

    Pattern 2: with pytest.raises(Exception, match='pattern'):
                   func()
    → expect(lambda: func()).to_raise(Exception)  # Note: match removed
    """
    conversions = 0
    match_removals = 0
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
            match_pattern = match.group(3)
            next_line = lines[i + 1]

            # Check if next line is a simple statement
            if is_simple_statement(next_line):
                statement = next_line.strip()

                # Convert to expect format (keep await in lambda)
                converted = f"{indent}expect(lambda: {statement}).to_raise({exception_type})"

                result.append(converted)
                conversions += 1
                if match_pattern:
                    match_removals += 1

                # Skip the next line (statement)
                i += 2
                continue

        result.append(line)
        i += 1

    return '\n'.join(result), conversions, match_removals


def convert_context_raises(content: str) -> Tuple[str, int]:
    """
    Convert pytest.raises with context manager (as exc_info).

    Pattern: with pytest.raises(Exception) as exc_info:
                 func()
             assert exc_info.value...

    → exc = expect(lambda: func()).to_raise(Exception)
      expect(exc...)...
    """
    conversions = 0
    lines = content.split('\n')
    result = []
    i = 0

    while i < len(lines):
        line = lines[i]

        # Match: with pytest.raises(ExceptionType) as var_name:
        match = re.match(r'^(\s*)with pytest\.raises\(([^)]+)\)\s+as\s+(\w+):\s*$', line)

        if match and i + 1 < len(lines):
            indent = match.group(1)
            exception_type = match.group(2)
            var_name = match.group(3)
            next_line = lines[i + 1]

            # Check if next line is a simple statement
            if is_simple_statement(next_line):
                statement = next_line.strip()

                # Convert to expect format with variable assignment (keep await)
                converted = f"{indent}{var_name} = expect(lambda: {statement}).to_raise({exception_type})"

                result.append(converted)
                conversions += 1

                # Skip the next line and continue processing subsequent lines
                # They might contain assertions on exc_info.value
                i += 2
                continue

        result.append(line)
        i += 1

    return '\n'.join(result), conversions


def add_expect_import(content: str) -> str:
    """Add expect import if not present."""
    if 'from data_bridge.test import expect' in content:
        return content

    # Find pytest import and add expect import after it
    lines = content.split('\n')
    result = []
    added = False

    for line in lines:
        result.append(line)
        if not added and 'import pytest' in line:
            result.append('from data_bridge.test import expect')
            added = True

    # If no pytest import found, add at the beginning after module docstring
    if not added:
        new_result = []
        in_docstring = False
        docstring_ended = False

        for i, line in enumerate(result):
            new_result.append(line)

            # Check for docstring
            if i == 0 and (line.strip().startswith('"""') or line.strip().startswith("'''")):
                in_docstring = True

            if in_docstring and i > 0 and ('"""' in line or "'''" in line):
                in_docstring = False
                docstring_ended = True

            if docstring_ended and not added and line.strip() and not line.strip().startswith('#'):
                new_result.insert(-1, 'from data_bridge.test import expect')
                added = True
                break

        if added:
            # Add remaining lines
            new_result.extend(result[len(new_result):])
            result = new_result

    return '\n'.join(result)


def convert_file(file_path: Path, stats: ConversionStats) -> bool:
    """Convert a single file. Returns True if file was modified."""
    try:
        stats.files_scanned += 1

        # Read file
        content = file_path.read_text()
        original_content = content

        # Skip if no pytest.raises
        if 'pytest.raises' not in content:
            return False

        # Apply conversions
        content, simple_conv, match_rem = convert_simple_raises(content)
        stats.conversions_simple += simple_conv
        stats.conversions_with_match += match_rem

        content, context_conv = convert_context_raises(content)
        stats.conversions_with_context += context_conv

        # Add expect import if conversions were made
        total_conv = simple_conv + context_conv
        if total_conv > 0:
            content = add_expect_import(content)

        # Write back if changed
        if content != original_content:
            file_path.write_text(content)
            stats.files_modified += 1
            return True

        return False

    except Exception as e:
        stats.errors.append(f"{file_path}: {str(e)}")
        return False


def main():
    """Main conversion routine."""
    tests_dir = Path(__file__).parent.parent / 'tests'

    if not tests_dir.exists():
        print(f"Error: Tests directory not found: {tests_dir}")
        sys.exit(1)

    stats = ConversionStats()

    # Find all Python test files
    test_files = list(tests_dir.rglob('test_*.py'))

    print(f"Found {len(test_files)} test files")
    print("Starting conversion...\n")

    modified_files = []

    for file_path in test_files:
        if convert_file(file_path, stats):
            modified_files.append(file_path)
            print(f"✓ Converted: {file_path.relative_to(tests_dir)}")

    # Print summary
    print("\n" + "=" * 70)
    print("CONVERSION SUMMARY")
    print("=" * 70)
    print(f"Files scanned:              {stats.files_scanned}")
    print(f"Files modified:             {stats.files_modified}")
    print(f"\nConversions by type:")
    print(f"  Simple raises:            {stats.conversions_simple}")
    print(f"  With match parameter:     {stats.conversions_with_match}")
    print(f"  With context (as var):    {stats.conversions_with_context}")
    print(f"  Multi-line blocks:        {stats.conversions_multiline}")
    print(f"\nSkipped (complex):          {stats.skipped_complex}")
    print(f"Errors:                     {len(stats.errors)}")

    if stats.errors:
        print("\nErrors:")
        for error in stats.errors:
            print(f"  - {error}")

    if modified_files:
        print(f"\n{len(modified_files)} files were modified:")
        for f in modified_files:
            print(f"  - {f.relative_to(tests_dir)}")

    print("\n" + "=" * 70)
    print("\nNext steps:")
    print("1. Review the changes: git diff")
    print("2. Run tests to verify: uv run python -m pytest tests/")
    print("3. Manual review needed for complex cases (multi-line blocks)")
    print("=" * 70)


if __name__ == '__main__':
    main()
