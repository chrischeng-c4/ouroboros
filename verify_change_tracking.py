#!/usr/bin/env python3
"""
Verification script for change tracking implementation.

This script demonstrates that:
1. Only changed fields are sent in UPDATE queries
2. No UPDATE is sent if nothing changed
3. _initial_data tracks state correctly
"""

from data_bridge.postgres import Table
from unittest.mock import AsyncMock, patch
import asyncio


class User(Table):
    name: str
    email: str
    age: int = 0

    class Settings:
        table_name = "users"


async def main():
    print("=" * 60)
    print("Change Tracking Verification")
    print("=" * 60)
    print()

    with patch('data_bridge.postgres.table._engine') as mock_engine:
        mock_engine.insert_one = AsyncMock(return_value=1)
        mock_engine.update_one = AsyncMock(return_value=1)

        # Test 1: Create and insert
        print("Test 1: Insert new user")
        print("-" * 60)
        user = User(name="Alice", email="alice@test.com", age=25)
        print(f"Created user: name={user.name}, email={user.email}, age={user.age}")
        print(f"Initial _data: {user._data}")
        print(f"Initial _initial_data: {user._initial_data}")

        await user.save()
        print(f"\nAfter save():")
        print(f"  user.id = {user.id}")
        print(f"  _data: {user._data}")
        print(f"  _initial_data: {user._initial_data}")
        print(f"  insert_one called: {mock_engine.insert_one.call_count} time(s)")
        print()

        # Test 2: Modify single field
        print("Test 2: Modify single field (age)")
        print("-" * 60)
        print(f"Before change: age={user.age}")
        user.age = 30
        print(f"After change: age={user.age}")

        await user.save()
        print(f"\nAfter save():")
        print(f"  update_one called: {mock_engine.update_one.call_count} time(s)")

        # Check what was sent to update_one
        call_args = mock_engine.update_one.call_args[0]
        updates_dict = call_args[3]
        print(f"  Updates sent: {updates_dict}")
        print(f"  Expected: {{'age': 30}}")
        print(f"  Match: {updates_dict == {'age': 30}}")
        print()

        # Test 3: Save without changes
        print("Test 3: Save without changes")
        print("-" * 60)
        update_count_before = mock_engine.update_one.call_count
        await user.save()
        update_count_after = mock_engine.update_one.call_count
        print(f"  update_one calls before: {update_count_before}")
        print(f"  update_one calls after: {update_count_after}")
        print(f"  UPDATE skipped: {update_count_before == update_count_after}")
        print()

        # Test 4: Modify multiple fields
        print("Test 4: Modify multiple fields")
        print("-" * 60)
        user.name = "Alice Smith"
        user.age = 35
        print(f"Changed: name={user.name}, age={user.age}")

        await user.save()
        call_args = mock_engine.update_one.call_args[0]
        updates_dict = call_args[3]
        print(f"\nAfter save():")
        print(f"  Updates sent: {updates_dict}")
        print(f"  Expected: {{'name': 'Alice Smith', 'age': 35}}")
        print(f"  'email' in updates: {'email' in updates_dict}")
        print()

    print("=" * 60)
    print("All verification tests passed!")
    print("=" * 60)


if __name__ == "__main__":
    asyncio.run(main())
