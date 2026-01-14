"""
Unit tests for Event System.

Tests the event system including EventType enum, EventDispatcher,
decorators, and AttributeEvents mixin without requiring a database.
"""
import pytest
import asyncio
from typing import List, Any
from unittest.mock import MagicMock, patch
from ouroboros.postgres.events import (
    EventType,
    EventDispatcher,
    EventRegistration,
    event,
    listens_for,
    listen,
    remove_listener,
    dispatch,
    dispatch_async,
    before_insert,
    after_insert,
    before_update,
    after_update,
    before_delete,
    after_delete,
    before_flush,
    after_commit,
    AttributeEvents,
)
from ouroboros.postgres import Table
from ouroboros.test import expect


# Test fixtures

@pytest.fixture
def fresh_dispatcher():
    """Create a fresh EventDispatcher for each test."""
    # Clear the singleton instance
    dispatcher = EventDispatcher()
    dispatcher.clear()
    return dispatcher


@pytest.fixture
def sample_table_class():
    """Sample Table class for event testing."""
    class User(Table):
        name: str
        email: str
        age: int = 0

        class Settings:
            table_name = "users"

    return User


@pytest.fixture
def sample_subclass(sample_table_class):
    """Sample subclass for inheritance testing."""
    class AdminUser(sample_table_class):
        admin_level: int = 1

        class Settings:
            table_name = "admin_users"

    return AdminUser


@pytest.fixture
def event_collector():
    """Helper to collect event calls."""
    class EventCollector:
        def __init__(self):
            self.events: List[tuple] = []

        def collect(self, *args, **kwargs):
            self.events.append((args, kwargs))
            return f"collected_{len(self.events)}"

        def clear(self):
            self.events.clear()

    return EventCollector()


# EventType Tests

class TestEventType:
    """Test EventType enum."""

    def test_crud_events_exist(self):
        """Test CRUD event types are defined."""
        expect(EventType.BEFORE_INSERT.value).to_equal("before_insert")
        expect(EventType.AFTER_INSERT.value).to_equal("after_insert")
        expect(EventType.BEFORE_UPDATE.value).to_equal("before_update")
        expect(EventType.AFTER_UPDATE.value).to_equal("after_update")
        expect(EventType.BEFORE_DELETE.value).to_equal("before_delete")
        expect(EventType.AFTER_DELETE.value).to_equal("after_delete")

    def test_session_events_exist(self):
        """Test session event types are defined."""
        expect(EventType.BEFORE_FLUSH.value).to_equal("before_flush")
        expect(EventType.AFTER_FLUSH.value).to_equal("after_flush")
        expect(EventType.AFTER_COMMIT.value).to_equal("after_commit")
        expect(EventType.AFTER_ROLLBACK.value).to_equal("after_rollback")

    def test_attribute_events_exist(self):
        """Test attribute event types are defined."""
        expect(EventType.ATTRIBUTE_SET.value).to_equal("attribute_set")
        expect(EventType.ATTRIBUTE_REMOVE.value).to_equal("attribute_remove")

    def test_load_events_exist(self):
        """Test load event types are defined."""
        expect(EventType.AFTER_LOAD.value).to_equal("after_load")
        expect(EventType.BEFORE_EXPIRE.value).to_equal("before_expire")

    def test_string_to_enum_conversion(self):
        """Test that string values can be converted to EventType."""
        expect(EventType("before_insert")).to_equal(EventType.BEFORE_INSERT)
        expect(EventType("after_update")).to_equal(EventType.AFTER_UPDATE)


# EventDispatcher Tests

class TestEventDispatcherSingleton:
    """Test EventDispatcher singleton pattern."""

    def test_singleton_instance(self):
        """Test EventDispatcher uses singleton pattern."""
        dispatcher1 = EventDispatcher()
        dispatcher2 = EventDispatcher()

        expect(dispatcher1 is dispatcher2).to_be_true()

    def test_singleton_state_shared(self, fresh_dispatcher):
        """Test singleton state is shared across instances."""
        def listener1(target):
            pass

        fresh_dispatcher.listen(None, EventType.BEFORE_INSERT, listener1)

        dispatcher2 = EventDispatcher()
        expect(dispatcher2.has_listeners(None, EventType.BEFORE_INSERT)).to_be_true()


class TestEventDispatcherRegister:
    """Test EventDispatcher.listen() method."""

    def test_register_global_listener(self, fresh_dispatcher):
        """Test registering a global listener (target=None)."""
        def my_listener(target):
            pass

        reg = fresh_dispatcher.listen(None, EventType.BEFORE_INSERT, my_listener)

        expect(isinstance(reg, EventRegistration)).to_be_true()
        expect(reg.event_type).to_equal(EventType.BEFORE_INSERT)
        expect(reg.target).to_equal(None)
        expect(reg.listener).to_equal(my_listener)

    def test_register_target_listener(self, fresh_dispatcher, sample_table_class):
        """Test registering a listener for specific target class."""
        def my_listener(target):
            pass

        reg = fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, my_listener
        )

        expect(reg.target).to_equal(sample_table_class)
        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_true()

    def test_register_with_string_event_type(self, fresh_dispatcher, sample_table_class):
        """Test registering with string event type."""
        def my_listener(target):
            pass

        reg = fresh_dispatcher.listen(
            sample_table_class, "before_insert", my_listener
        )

        expect(reg.event_type).to_equal(EventType.BEFORE_INSERT)

    def test_register_with_propagate_false(self, fresh_dispatcher, sample_table_class):
        """Test registering with propagate=False."""
        def my_listener(target):
            pass

        reg = fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, my_listener, propagate=False
        )

        expect(reg.propagate).to_equal(False)

    def test_register_with_once_true(self, fresh_dispatcher, sample_table_class):
        """Test registering with once=True."""
        def my_listener(target):
            pass

        reg = fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, my_listener, once=True
        )

        expect(reg.once).to_equal(True)

    def test_register_with_priority(self, fresh_dispatcher, sample_table_class):
        """Test registering with custom priority."""
        def my_listener(target):
            pass

        reg = fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, my_listener, priority=10
        )

        expect(reg.priority).to_equal(10)


class TestEventDispatcherDispatch:
    """Test EventDispatcher.dispatch() method."""

    def test_dispatch_to_global_listener(self, fresh_dispatcher, sample_table_class):
        """Test dispatching to global listeners."""
        called = []

        def my_listener(target):
            called.append(target)

        fresh_dispatcher.listen(None, EventType.BEFORE_INSERT, my_listener)

        user = sample_table_class(name="Test", email="test@example.com")
        results = fresh_dispatcher.dispatch(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(1)
        expect(called[0]).to_equal(user)

    def test_dispatch_to_target_listener(self, fresh_dispatcher, sample_table_class):
        """Test dispatching to target-specific listeners."""
        called = []

        def my_listener(target):
            called.append(target)

        fresh_dispatcher.listen(sample_table_class, EventType.BEFORE_INSERT, my_listener)

        user = sample_table_class(name="Test", email="test@example.com")
        fresh_dispatcher.dispatch(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(1)

    def test_dispatch_with_string_event_type(self, fresh_dispatcher, sample_table_class):
        """Test dispatching with string event type."""
        called = []

        def my_listener(target):
            called.append(target)

        fresh_dispatcher.listen(sample_table_class, "before_insert", my_listener)

        user = sample_table_class(name="Test", email="test@example.com")
        fresh_dispatcher.dispatch("before_insert", user)

        expect(len(called)).to_equal(1)

    def test_dispatch_passes_extra_args(self, fresh_dispatcher, sample_table_class):
        """Test dispatch passes additional args to listeners."""
        received_args = []

        def my_listener(target, *args, **kwargs):
            received_args.append((args, kwargs))

        fresh_dispatcher.listen(sample_table_class, EventType.BEFORE_UPDATE, my_listener)

        user = sample_table_class(name="Test", email="test@example.com")
        fresh_dispatcher.dispatch(
            EventType.BEFORE_UPDATE, user, "arg1", "arg2", key="value"
        )

        expect(len(received_args)).to_equal(1)
        expect(received_args[0][0]).to_equal(("arg1", "arg2"))
        expect(received_args[0][1]).to_equal({"key": "value"})

    def test_dispatch_returns_results(self, fresh_dispatcher, sample_table_class):
        """Test dispatch returns list of listener return values."""
        def listener1(target):
            return "result1"

        def listener2(target):
            return "result2"

        fresh_dispatcher.listen(sample_table_class, EventType.BEFORE_INSERT, listener1)
        fresh_dispatcher.listen(sample_table_class, EventType.BEFORE_INSERT, listener2)

        user = sample_table_class(name="Test", email="test@example.com")
        results = fresh_dispatcher.dispatch(EventType.BEFORE_INSERT, user)

        expect(len(results)).to_equal(2)
        expect("result1" in results).to_be_true()
        expect("result2" in results).to_be_true()

    def test_dispatch_once_removes_listener(self, fresh_dispatcher, sample_table_class):
        """Test once=True listeners are removed after firing."""
        call_count = [0]

        def my_listener(target):
            call_count[0] += 1

        fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, my_listener, once=True
        )

        user = sample_table_class(name="Test", email="test@example.com")
        fresh_dispatcher.dispatch(EventType.BEFORE_INSERT, user)
        fresh_dispatcher.dispatch(EventType.BEFORE_INSERT, user)

        expect(call_count[0]).to_equal(1)

    def test_dispatch_handles_listener_errors(self, fresh_dispatcher, sample_table_class):
        """Test dispatch continues even if listener raises error."""
        called = []

        def error_listener(target):
            raise ValueError("Test error")

        def success_listener(target):
            called.append(target)

        fresh_dispatcher.listen(sample_table_class, EventType.BEFORE_INSERT, error_listener)
        fresh_dispatcher.listen(sample_table_class, EventType.BEFORE_INSERT, success_listener)

        user = sample_table_class(name="Test", email="test@example.com")

        # Should not raise, just warn
        with pytest.warns(UserWarning, match="Event listener error"):
            fresh_dispatcher.dispatch(EventType.BEFORE_INSERT, user)

        # Second listener should still be called
        expect(len(called)).to_equal(1)


class TestEventDispatcherUnregister:
    """Test EventDispatcher.remove() method."""

    def test_remove_global_listener(self, fresh_dispatcher):
        """Test removing a global listener."""
        def my_listener(target):
            pass

        reg = fresh_dispatcher.listen(None, EventType.BEFORE_INSERT, my_listener)

        result = fresh_dispatcher.remove(reg)

        expect(result).to_be_true()
        expect(fresh_dispatcher.has_listeners(None, EventType.BEFORE_INSERT)).to_be_false()

    def test_remove_target_listener(self, fresh_dispatcher, sample_table_class):
        """Test removing a target-specific listener."""
        def my_listener(target):
            pass

        reg = fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, my_listener
        )

        result = fresh_dispatcher.remove(reg)

        expect(result).to_be_true()
        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_false()

    def test_remove_nonexistent_listener(self, fresh_dispatcher, sample_table_class):
        """Test removing a listener that doesn't exist returns False."""
        def my_listener(target):
            pass

        reg = EventRegistration(
            event_type=EventType.BEFORE_INSERT,
            target=sample_table_class,
            listener=my_listener,
        )

        result = fresh_dispatcher.remove(reg)

        expect(result).to_be_false()

    def test_remove_stops_dispatch(self, fresh_dispatcher, sample_table_class):
        """Test removed listeners are not called on dispatch."""
        called = []

        def my_listener(target):
            called.append(target)

        reg = fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, my_listener
        )

        fresh_dispatcher.remove(reg)

        user = sample_table_class(name="Test", email="test@example.com")
        fresh_dispatcher.dispatch(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(0)


class TestEventDispatcherGetListeners:
    """Test EventDispatcher._get_listeners() method."""

    def test_get_listeners_includes_global(self, fresh_dispatcher, sample_table_class):
        """Test _get_listeners includes global listeners."""
        def global_listener(target):
            pass

        fresh_dispatcher.listen(None, EventType.BEFORE_INSERT, global_listener)

        listeners = fresh_dispatcher._get_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )

        expect(len(listeners)).to_equal(1)
        expect(listeners[0].listener).to_equal(global_listener)

    def test_get_listeners_includes_target(self, fresh_dispatcher, sample_table_class):
        """Test _get_listeners includes target-specific listeners."""
        def target_listener(target):
            pass

        fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, target_listener
        )

        listeners = fresh_dispatcher._get_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )

        expect(len(listeners)).to_equal(1)
        expect(listeners[0].listener).to_equal(target_listener)

    def test_get_listeners_includes_inherited(
        self, fresh_dispatcher, sample_table_class, sample_subclass
    ):
        """Test _get_listeners includes listeners from parent classes."""
        def parent_listener(target):
            pass

        fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, parent_listener, propagate=True
        )

        listeners = fresh_dispatcher._get_listeners(
            sample_subclass, EventType.BEFORE_INSERT
        )

        expect(len(listeners)).to_equal(1)
        expect(listeners[0].listener).to_equal(parent_listener)

    def test_get_listeners_respects_propagate_false(
        self, fresh_dispatcher, sample_table_class, sample_subclass
    ):
        """Test propagate=False prevents inheritance."""
        def parent_listener(target):
            pass

        fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, parent_listener, propagate=False
        )

        listeners = fresh_dispatcher._get_listeners(
            sample_subclass, EventType.BEFORE_INSERT
        )

        expect(len(listeners)).to_equal(0)

    def test_get_listeners_sorted_by_priority(self, fresh_dispatcher, sample_table_class):
        """Test listeners are sorted by priority (highest first)."""
        def low_priority(target):
            pass

        def high_priority(target):
            pass

        def medium_priority(target):
            pass

        fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, low_priority, priority=1
        )
        fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, high_priority, priority=10
        )
        fresh_dispatcher.listen(
            sample_table_class, EventType.BEFORE_INSERT, medium_priority, priority=5
        )

        listeners = fresh_dispatcher._get_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )

        expect(listeners[0].listener).to_equal(high_priority)
        expect(listeners[1].listener).to_equal(medium_priority)
        expect(listeners[2].listener).to_equal(low_priority)


class TestEventDispatcherClear:
    """Test EventDispatcher.clear() method."""

    def test_clear_all_listeners(self, fresh_dispatcher, sample_table_class):
        """Test clear() removes all listeners."""
        def listener1(target):
            pass

        def listener2(target):
            pass

        fresh_dispatcher.listen(None, EventType.BEFORE_INSERT, listener1)
        fresh_dispatcher.listen(sample_table_class, EventType.BEFORE_UPDATE, listener2)

        fresh_dispatcher.clear()

        expect(fresh_dispatcher.has_listeners(None, EventType.BEFORE_INSERT)).to_be_false()
        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_UPDATE
        )).to_be_false()

    def test_clear_target_specific(self, fresh_dispatcher, sample_table_class):
        """Test clear(target) only removes listeners for that target."""
        def listener1(target):
            pass

        def listener2(target):
            pass

        fresh_dispatcher.listen(None, EventType.BEFORE_INSERT, listener1)
        fresh_dispatcher.listen(sample_table_class, EventType.BEFORE_UPDATE, listener2)

        fresh_dispatcher.clear(sample_table_class)

        expect(fresh_dispatcher.has_listeners(None, EventType.BEFORE_INSERT)).to_be_true()
        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_UPDATE
        )).to_be_false()


# Decorator Tests

class TestListensForDecorator:
    """Test @listens_for decorator."""

    def test_listens_for_registers_listener(self, fresh_dispatcher, sample_table_class):
        """Test @listens_for decorator registers the function."""
        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_true()

    def test_listens_for_with_string_event(self, fresh_dispatcher, sample_table_class):
        """Test @listens_for with string event type."""
        @listens_for(sample_table_class, "before_insert")
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_true()

    def test_listens_for_global(self, fresh_dispatcher):
        """Test @listens_for with None target for global listener."""
        @listens_for(None, EventType.AFTER_COMMIT)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(None, EventType.AFTER_COMMIT)).to_be_true()

    def test_listens_for_returns_function(self, fresh_dispatcher, sample_table_class):
        """Test @listens_for returns the original function."""
        def my_listener(target):
            return "test_value"

        decorated = listens_for(sample_table_class, EventType.BEFORE_INSERT)(my_listener)

        expect(decorated).to_equal(my_listener)
        expect(decorated(None)).to_equal("test_value")

    def test_listens_for_with_options(self, fresh_dispatcher, sample_table_class):
        """Test @listens_for with propagate, once, and priority options."""
        @listens_for(
            sample_table_class, EventType.BEFORE_INSERT,
            propagate=False, once=True, priority=5
        )
        def my_listener(target):
            pass

        listeners = fresh_dispatcher._get_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )

        expect(len(listeners)).to_equal(1)
        expect(listeners[0].propagate).to_equal(False)
        expect(listeners[0].once).to_equal(True)
        expect(listeners[0].priority).to_equal(5)


class TestListenFunction:
    """Test listen() function."""

    def test_listen_function_registers_listener(self, fresh_dispatcher, sample_table_class):
        """Test listen() function registers a listener."""
        def my_listener(target):
            pass

        reg = listen(sample_table_class, EventType.BEFORE_INSERT, my_listener)

        expect(isinstance(reg, EventRegistration)).to_be_true()
        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_true()

    def test_listen_function_returns_registration(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test listen() returns EventRegistration for removal."""
        def my_listener(target):
            pass

        reg = listen(sample_table_class, EventType.BEFORE_INSERT, my_listener)

        result = remove_listener(reg)

        expect(result).to_be_true()
        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_false()


class TestConvenienceDecorators:
    """Test convenience decorators (before_insert, after_update, etc.)."""

    def test_before_insert_decorator(self, fresh_dispatcher, sample_table_class):
        """Test @before_insert decorator."""
        @before_insert(sample_table_class)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_true()

    def test_after_insert_decorator(self, fresh_dispatcher, sample_table_class):
        """Test @after_insert decorator."""
        @after_insert(sample_table_class)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.AFTER_INSERT
        )).to_be_true()

    def test_before_update_decorator(self, fresh_dispatcher, sample_table_class):
        """Test @before_update decorator."""
        @before_update(sample_table_class)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_UPDATE
        )).to_be_true()

    def test_after_update_decorator(self, fresh_dispatcher, sample_table_class):
        """Test @after_update decorator."""
        @after_update(sample_table_class)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.AFTER_UPDATE
        )).to_be_true()

    def test_before_delete_decorator(self, fresh_dispatcher, sample_table_class):
        """Test @before_delete decorator."""
        @before_delete(sample_table_class)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_DELETE
        )).to_be_true()

    def test_after_delete_decorator(self, fresh_dispatcher, sample_table_class):
        """Test @after_delete decorator."""
        @after_delete(sample_table_class)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.AFTER_DELETE
        )).to_be_true()

    def test_before_flush_decorator(self, fresh_dispatcher):
        """Test @before_flush decorator (global)."""
        @before_flush()
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(None, EventType.BEFORE_FLUSH)).to_be_true()

    def test_after_commit_decorator(self, fresh_dispatcher):
        """Test @after_commit decorator (global)."""
        @after_commit()
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(None, EventType.AFTER_COMMIT)).to_be_true()

    def test_convenience_decorator_with_options(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test convenience decorators accept options."""
        @before_insert(sample_table_class, priority=10, once=True)
        def my_listener(target):
            pass

        listeners = fresh_dispatcher._get_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )

        expect(len(listeners)).to_equal(1)
        expect(listeners[0].priority).to_equal(10)
        expect(listeners[0].once).to_equal(True)


class TestDispatchFunctions:
    """Test dispatch() and dispatch_async() functions."""

    def test_dispatch_function(self, fresh_dispatcher, sample_table_class):
        """Test dispatch() function calls listeners."""
        called = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def my_listener(target):
            called.append(target)

        user = sample_table_class(name="Test", email="test@example.com")
        results = dispatch(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(1)
        expect(isinstance(results, list)).to_be_true()

    @pytest.mark.asyncio
    async def test_dispatch_async_function(self, fresh_dispatcher, sample_table_class):
        """Test dispatch_async() function."""
        called = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def my_listener(target):
            called.append(target)

        user = sample_table_class(name="Test", email="test@example.com")
        results = await dispatch_async(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(1)
        expect(isinstance(results, list)).to_be_true()

    @pytest.mark.asyncio
    async def test_dispatch_async_with_async_listener(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test dispatch_async() handles async listeners."""
        called = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        async def my_async_listener(target):
            await asyncio.sleep(0.01)
            called.append(target)
            return "async_result"

        user = sample_table_class(name="Test", email="test@example.com")
        results = await dispatch_async(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(1)
        expect("async_result" in results).to_be_true()

    @pytest.mark.asyncio
    async def test_dispatch_async_mixed_listeners(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test dispatch_async() handles mix of sync and async listeners."""
        called = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=10)
        def sync_listener(target):
            called.append("sync")

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=5)
        async def async_listener(target):
            await asyncio.sleep(0.01)
            called.append("async")

        user = sample_table_class(name="Test", email="test@example.com")
        await dispatch_async(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(2)
        expect(called[0]).to_equal("sync")  # Higher priority first
        expect(called[1]).to_equal("async")


# Multiple Listeners Tests

class TestMultipleListeners:
    """Test multiple listeners for same event."""

    def test_multiple_listeners_all_called(self, fresh_dispatcher, sample_table_class):
        """Test all registered listeners are called."""
        called = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def listener1(target):
            called.append(1)

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def listener2(target):
            called.append(2)

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def listener3(target):
            called.append(3)

        user = sample_table_class(name="Test", email="test@example.com")
        dispatch(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(3)
        expect(1 in called).to_be_true()
        expect(2 in called).to_be_true()
        expect(3 in called).to_be_true()

    def test_global_and_target_listeners_both_called(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test both global and target-specific listeners are called."""
        called = []

        @listens_for(None, EventType.BEFORE_INSERT)
        def global_listener(target):
            called.append("global")

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def target_listener(target):
            called.append("target")

        user = sample_table_class(name="Test", email="test@example.com")
        dispatch(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(2)
        expect("global" in called).to_be_true()
        expect("target" in called).to_be_true()


# Priority Ordering Tests

class TestListenerPriority:
    """Test listener priority ordering."""

    def test_priority_order_respected(self, fresh_dispatcher, sample_table_class):
        """Test listeners are called in priority order (highest first)."""
        call_order = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=1)
        def low_priority(target):
            call_order.append("low")

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=10)
        def high_priority(target):
            call_order.append("high")

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=5)
        def medium_priority(target):
            call_order.append("medium")

        user = sample_table_class(name="Test", email="test@example.com")
        dispatch(EventType.BEFORE_INSERT, user)

        expect(call_order).to_equal(["high", "medium", "low"])

    def test_default_priority_zero(self, fresh_dispatcher, sample_table_class):
        """Test default priority is 0."""
        call_order = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=1)
        def high_priority(target):
            call_order.append("high")

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)  # default priority=0
        def default_priority(target):
            call_order.append("default")

        user = sample_table_class(name="Test", email="test@example.com")
        dispatch(EventType.BEFORE_INSERT, user)

        expect(call_order).to_equal(["high", "default"])


# Error Handling Tests

class TestErrorHandling:
    """Test error handling in listeners."""

    def test_error_in_listener_does_not_stop_others(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test error in one listener doesn't stop other listeners."""
        called = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=10)
        def error_listener(target):
            raise ValueError("Test error")

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=5)
        def success_listener(target):
            called.append("success")

        user = sample_table_class(name="Test", email="test@example.com")

        with pytest.warns(UserWarning, match="Event listener error"):
            dispatch(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(1)
        expect(called[0]).to_equal("success")

    def test_multiple_errors_all_warned(self, fresh_dispatcher, sample_table_class):
        """Test multiple errors all generate warnings."""
        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def error_listener1(target):
            raise ValueError("Error 1")

        @listens_for(sample_table_class, EventType.BEFORE_INSERT)
        def error_listener2(target):
            raise TypeError("Error 2")

        user = sample_table_class(name="Test", email="test@example.com")

        with pytest.warns(UserWarning) as warnings:
            dispatch(EventType.BEFORE_INSERT, user)

        expect(len(warnings) >= 2).to_be_true()

    @pytest.mark.asyncio
    async def test_async_error_handling(self, fresh_dispatcher, sample_table_class):
        """Test error handling in async dispatch."""
        called = []

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=10)
        async def error_listener(target):
            raise ValueError("Async error")

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, priority=5)
        async def success_listener(target):
            called.append("success")

        user = sample_table_class(name="Test", email="test@example.com")

        with pytest.warns(UserWarning, match="Event listener error"):
            await dispatch_async(EventType.BEFORE_INSERT, user)

        expect(len(called)).to_equal(1)

    @pytest.mark.asyncio
    async def test_dispatch_async_with_string_event_type(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test dispatch_async with string event type."""
        called = []

        @listens_for(sample_table_class, "before_insert")
        async def my_listener(target):
            called.append(target)

        user = sample_table_class(name="Test", email="test@example.com")
        await dispatch_async("before_insert", user)

        expect(len(called)).to_equal(1)

    @pytest.mark.asyncio
    async def test_dispatch_async_once_removes_listener(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test once=True listeners are removed after async dispatch."""
        call_count = [0]

        @listens_for(sample_table_class, EventType.BEFORE_INSERT, once=True)
        async def my_listener(target):
            call_count[0] += 1

        user = sample_table_class(name="Test", email="test@example.com")
        await dispatch_async(EventType.BEFORE_INSERT, user)
        await dispatch_async(EventType.BEFORE_INSERT, user)

        expect(call_count[0]).to_equal(1)


# AttributeEvents Mixin Tests

class TestAttributeEventsMixin:
    """Test AttributeEvents mixin class."""

    def test_attribute_events_mixin_exists(self):
        """Test AttributeEvents can be mixed into a class."""
        class TestModel(AttributeEvents):
            name: str
            age: int

        model = TestModel()
        expect(hasattr(model, '_tracking_enabled')).to_be_true()

    def test_setattr_tracking_disabled_by_default(self, fresh_dispatcher):
        """Test attribute tracking is disabled by default."""
        called = []

        class TestModel(AttributeEvents):
            def __init__(self):
                self.name = "initial"

        @listens_for(TestModel, EventType.ATTRIBUTE_SET)
        def on_change(target, key, old, new):
            called.append((key, old, new))

        model = TestModel()
        model.name = "changed"

        expect(len(called)).to_equal(0)

    def test_enable_tracking_dispatches_events(self, fresh_dispatcher):
        """Test enable_tracking() enables attribute change events."""
        called = []

        class TestModel(AttributeEvents):
            def __init__(self):
                self.name = "initial"
                self.age = 25

        @listens_for(TestModel, EventType.ATTRIBUTE_SET)
        def on_change(target, key, old, new):
            called.append((key, old, new))

        model = TestModel()
        model.enable_tracking()
        model.name = "changed"

        expect(len(called)).to_equal(1)
        expect(called[0]).to_equal(("name", "initial", "changed"))

    def test_tracking_only_fires_on_change(self, fresh_dispatcher):
        """Test attribute events only fire when value actually changes."""
        called = []

        class TestModel(AttributeEvents):
            def __init__(self):
                self.name = "test"

        @listens_for(TestModel, EventType.ATTRIBUTE_SET)
        def on_change(target, key, old, new):
            called.append((key, old, new))

        model = TestModel()
        model.enable_tracking()
        model.name = "test"  # Same value

        expect(len(called)).to_equal(0)

    def test_disable_tracking_stops_events(self, fresh_dispatcher):
        """Test disable_tracking() stops attribute change events."""
        called = []

        class TestModel(AttributeEvents):
            def __init__(self):
                self.name = "initial"

        @listens_for(TestModel, EventType.ATTRIBUTE_SET)
        def on_change(target, key, old, new):
            called.append((key, old, new))

        model = TestModel()
        model.enable_tracking()
        model.disable_tracking()
        model.name = "changed"

        expect(len(called)).to_equal(0)

    def test_private_attributes_not_tracked(self, fresh_dispatcher):
        """Test private attributes (starting with _) are not tracked."""
        called = []

        class TestModel(AttributeEvents):
            def __init__(self):
                self._private = "initial"
                self.public = "initial"

        @listens_for(TestModel, EventType.ATTRIBUTE_SET)
        def on_change(target, key, old, new):
            called.append((key, old, new))

        model = TestModel()
        model.enable_tracking()
        model._private = "changed"
        model.public = "changed"

        expect(len(called)).to_equal(1)
        expect(called[0][0]).to_equal("public")

    def test_multiple_attribute_changes_tracked(self, fresh_dispatcher):
        """Test multiple attribute changes are all tracked."""
        called = []

        class TestModel(AttributeEvents):
            def __init__(self):
                self.name = "initial"
                self.age = 25
                self.city = "NYC"

        @listens_for(TestModel, EventType.ATTRIBUTE_SET)
        def on_change(target, key, old, new):
            called.append(key)

        model = TestModel()
        model.enable_tracking()
        model.name = "changed"
        model.age = 30
        model.city = "LA"

        expect(len(called)).to_equal(3)
        expect("name" in called).to_be_true()
        expect("age" in called).to_be_true()
        expect("city" in called).to_be_true()

    def test_attribute_events_with_table(self, fresh_dispatcher):
        """Test AttributeEvents can be combined with Table class.

        Note: When combining Table and AttributeEvents, the Table's __setattr__
        may override AttributeEvents' tracking. This test verifies the mixin
        can be used together with Table without errors, even though tracking
        may require integration with Table's _data mechanism.

        For full attribute tracking on Table instances, the Table class itself
        would need to call event.dispatch() in its __setattr__ method.
        """
        class User(Table, AttributeEvents):
            name: str
            email: str

            class Settings:
                table_name = "users"

        # Verify the class can be created and used
        user = User(name="Test", email="test@example.com")
        expect(hasattr(user, '_tracking_enabled')).to_be_true()

        user.enable_tracking()
        expect(user._tracking_enabled).to_be_true()

        user.disable_tracking()
        expect(user._tracking_enabled).to_be_false()


# Integration Tests

class TestEventSystemIntegration:
    """Test event system integration scenarios."""

    def test_before_insert_modifies_instance(self, fresh_dispatcher, sample_table_class):
        """Test before_insert can modify the instance being inserted."""
        @before_insert(sample_table_class)
        def set_defaults(target):
            if not hasattr(target, 'age') or target.age == 0:
                target.age = 18

        user = sample_table_class(name="Test", email="test@example.com")
        dispatch(EventType.BEFORE_INSERT, user)

        expect(user.age).to_equal(18)

    def test_inheritance_propagation(
        self, fresh_dispatcher, sample_table_class, sample_subclass
    ):
        """Test event listeners propagate to subclasses by default."""
        called = []

        @before_insert(sample_table_class)
        def parent_listener(target):
            called.append(type(target).__name__)

        admin = sample_subclass(name="Admin", email="admin@example.com")
        dispatch(EventType.BEFORE_INSERT, admin)

        expect(len(called)).to_equal(1)
        expect(called[0]).to_equal("AdminUser")

    def test_no_propagation_blocks_inheritance(
        self, fresh_dispatcher, sample_table_class, sample_subclass
    ):
        """Test propagate=False blocks event from firing on subclasses."""
        called = []

        @before_insert(sample_table_class, propagate=False)
        def parent_listener(target):
            called.append(type(target).__name__)

        admin = sample_subclass(name="Admin", email="admin@example.com")
        dispatch(EventType.BEFORE_INSERT, admin)

        expect(len(called)).to_equal(0)

    def test_complex_event_chain(self, fresh_dispatcher, sample_table_class):
        """Test complex chain of events with different priorities."""
        execution_log = []

        @before_insert(sample_table_class, priority=100)
        def validate(target):
            execution_log.append("validate")

        @before_insert(sample_table_class, priority=50)
        def set_defaults(target):
            execution_log.append("defaults")

        @before_insert(sample_table_class, priority=10)
        def log_event(target):
            execution_log.append("log")

        user = sample_table_class(name="Test", email="test@example.com")
        dispatch(EventType.BEFORE_INSERT, user)

        expect(execution_log).to_equal(["validate", "defaults", "log"])

    def test_listener_can_return_values(self, fresh_dispatcher, sample_table_class):
        """Test listener return values are collected."""
        @before_insert(sample_table_class, priority=10)
        def validator(target):
            return {"valid": True}

        @before_insert(sample_table_class, priority=5)
        def logger(target):
            return {"logged": True}

        user = sample_table_class(name="Test", email="test@example.com")
        results = dispatch(EventType.BEFORE_INSERT, user)

        expect(len(results)).to_equal(2)
        expect({"valid": True} in results).to_be_true()
        expect({"logged": True} in results).to_be_true()

    def test_has_listeners_check(self, fresh_dispatcher, sample_table_class):
        """Test has_listeners() correctly reports listener presence."""
        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_false()

        @before_insert(sample_table_class)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, EventType.BEFORE_INSERT
        )).to_be_true()

    def test_has_listeners_with_string_event_type(
        self, fresh_dispatcher, sample_table_class
    ):
        """Test has_listeners() works with string event type."""
        expect(fresh_dispatcher.has_listeners(
            sample_table_class, "before_insert"
        )).to_be_false()

        @before_insert(sample_table_class)
        def my_listener(target):
            pass

        expect(fresh_dispatcher.has_listeners(
            sample_table_class, "before_insert"
        )).to_be_true()

    @pytest.mark.asyncio
    async def test_full_async_workflow(self, fresh_dispatcher, sample_table_class):
        """Test complete async event workflow."""
        workflow = []

        @before_insert(sample_table_class, priority=10)
        async def async_validate(target):
            await asyncio.sleep(0.01)
            workflow.append("validate")
            return True

        @before_insert(sample_table_class, priority=5)
        def sync_log(target):
            workflow.append("log")

        @after_insert(sample_table_class)
        async def async_notify(target):
            await asyncio.sleep(0.01)
            workflow.append("notify")

        user = sample_table_class(name="Test", email="test@example.com")

        results1 = await dispatch_async(EventType.BEFORE_INSERT, user)
        results2 = await dispatch_async(EventType.AFTER_INSERT, user)

        expect(workflow).to_equal(["validate", "log", "notify"])
        expect(True in results1).to_be_true()
