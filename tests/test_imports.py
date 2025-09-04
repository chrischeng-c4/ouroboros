"""Test that all imports work correctly after reorganization."""


def test_base_imports() -> None:
    """Test base package imports."""
    from data_bridge.base.backends.async_ import AsyncBackend
    from data_bridge.base.backends.sync import SyncBackend
    from data_bridge.base.fields import (
        CompoundExpression,
        Field,
        QueryExpression,
    )
    from data_bridge.base.manager import BaseManager
    from data_bridge.base.model import BaseModel
    from data_bridge.base.query import BaseQuery

    assert Field is not None
    assert QueryExpression is not None
    assert CompoundExpression is not None
    assert BaseModel is not None
    assert BaseManager is not None
    assert BaseQuery is not None
    assert SyncBackend is not None
    assert AsyncBackend is not None


def test_mongodb_sync_imports() -> None:
    """Test MongoDB synchronous imports."""
    from data_bridge.mongo.sync import (
        Document,
        Field,
        MongoCollection,
        MongoQuery,
        MongoSyncBackend,
        ObjectIdField,
    )

    assert Document is not None
    assert Field is not None
    assert ObjectIdField is not None
    assert MongoCollection is not None
    assert MongoQuery is not None
    assert MongoSyncBackend is not None


def test_mongodb_async_imports() -> None:
    """Test MongoDB asynchronous imports."""
    from data_bridge.mongo.async_ import (
        AsyncDocument,
        AsyncMongoCollection,
        AsyncMongoQuery,
        Field,
        MongoAsyncBackend,
        ObjectIdField,
    )

    assert AsyncDocument is not None
    assert Field is not None
    assert ObjectIdField is not None
    assert AsyncMongoCollection is not None
    assert AsyncMongoQuery is not None
    assert MongoAsyncBackend is not None


def test_redis_sync_imports() -> None:
    """Test Redis synchronous imports."""
    from data_bridge.redis.sync import (
        Field,
        HashModel,
        JSONModel,
        RedisManager,
        RedisSyncBackend,
        TTLField,
    )

    assert HashModel is not None
    assert JSONModel is not None
    assert Field is not None
    assert TTLField is not None
    assert RedisManager is not None
    assert RedisSyncBackend is not None


def test_redis_async_imports() -> None:
    """Test Redis asynchronous imports."""
    from data_bridge.redis.async_ import (
        AsyncHashModel,
        AsyncJSONModel,
        AsyncRedisManager,
        Field,
        RedisAsyncBackend,
        TTLField,
    )

    assert AsyncHashModel is not None
    assert AsyncJSONModel is not None
    assert Field is not None
    assert TTLField is not None
    assert AsyncRedisManager is not None
    assert RedisAsyncBackend is not None


def test_main_package_imports() -> None:
    """Test main package imports."""
    from data_bridge import CompoundExpression, Field, QueryExpression

    assert Field is not None
    assert QueryExpression is not None
    assert CompoundExpression is not None


def test_mongodb_translator() -> None:
    """Test MongoDB query translator."""
    from data_bridge.base.fields import QueryExpression
    from data_bridge.mongo.translator import MongoQueryTranslator

    # Test basic query translation
    expr = QueryExpression(field="age", operator="gte", value=18)
    result = MongoQueryTranslator.translate([expr])
    expected = {"age": {"$gte": 18}}
    assert result == expected


def test_redis_key_patterns() -> None:
    """Test Redis key pattern utilities."""
    from data_bridge.redis.key_patterns import RedisKeyPattern

    # Test key building
    key = RedisKeyPattern.build_key("user:", "123")
    assert key == "user:123"

    # Test key parsing
    parsed = RedisKeyPattern.parse_key("user:123", "user:")
    assert parsed == "123"

    # Test pattern building
    pattern = RedisKeyPattern.build_pattern("user:")
    assert pattern == "user:*"


if __name__ == "__main__":
    import traceback

    # Run the tests individually to see which ones pass/fail
    test_functions = [
        test_base_imports,
        test_mongodb_sync_imports,
        test_mongodb_async_imports,
        test_redis_sync_imports,
        test_redis_async_imports,
        test_main_package_imports,
        test_mongodb_translator,
        test_redis_key_patterns,
    ]

    for test_func in test_functions:
        try:
            test_func()
            print(f"✓ {test_func.__name__}")  # noqa: T201
        except Exception as e:
            print(f"❌ {test_func.__name__}: {e}")  # noqa: T201
            traceback.print_exc()

