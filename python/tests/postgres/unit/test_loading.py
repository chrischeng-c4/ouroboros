"""
Unit tests for loading strategies module.

Tests LoadingStrategy enum, LoadingConfig dataclass, factory functions,
LazyLoadingProxy, DeferredColumn, RelationshipLoader, and custom exceptions.
"""
import pytest
from unittest.mock import AsyncMock, Mock, patch
from ouroboros.test import expect
from ouroboros.postgres.loading import (
    LoadingStrategy,
    LoadingConfig,
    lazy,
    joined,
    subquery,
    selectinload,
    noload,
    raiseload,
    defer,
    undefer,
    LazyLoadingProxy,
    DeferredColumn,
    RelationshipLoader,
    LazyLoadError,
    SQLGenerationError,
)


class TestLoadingStrategy:
    """Test LoadingStrategy enum."""

    def test_all_strategies_exist(self):
        """Test all 7 loading strategies are defined."""
        expect(LoadingStrategy.LAZY.value).to_equal("lazy")
        expect(LoadingStrategy.JOINED.value).to_equal("joined")
        expect(LoadingStrategy.SUBQUERY.value).to_equal("subquery")
        expect(LoadingStrategy.SELECTIN.value).to_equal("selectin")
        expect(LoadingStrategy.NOLOAD.value).to_equal("noload")
        expect(LoadingStrategy.RAISE.value).to_equal("raise")
        expect(LoadingStrategy.RAISE_ON_SQL.value).to_equal("raise_on_sql")

    def test_enum_values_are_strings(self):
        """Test all enum values are strings."""
        for strategy in LoadingStrategy:
            expect(isinstance(strategy.value, str)).to_be_true()

    def test_enum_members_count(self):
        """Test exactly 7 strategies exist."""
        expect(len(LoadingStrategy)).to_equal(7)


class TestLoadingConfig:
    """Test LoadingConfig dataclass."""

    def test_loading_config_creation(self):
        """Test LoadingConfig can be created with required fields."""
        config = LoadingConfig(strategy=LoadingStrategy.LAZY)

        expect(config.strategy).to_equal(LoadingStrategy.LAZY)
        expect(config.sql_only).to_be_false()
        expect(config.deferred_columns).to_equal([])
        expect(config.innerjoin).to_be_false()
        expect(config.columns).to_be_none()

    def test_loading_config_with_all_fields(self):
        """Test LoadingConfig with all fields set."""
        config = LoadingConfig(
            strategy=LoadingStrategy.JOINED,
            sql_only=True,
            deferred_columns=["content", "metadata"],
            innerjoin=True,
            columns=["id", "name"]
        )

        expect(config.strategy).to_equal(LoadingStrategy.JOINED)
        expect(config.sql_only).to_be_true()
        expect(config.deferred_columns).to_equal(["content", "metadata"])
        expect(config.innerjoin).to_be_true()
        expect(config.columns).to_equal(["id", "name"])

    def test_loading_config_defaults(self):
        """Test LoadingConfig default values."""
        config = LoadingConfig(strategy=LoadingStrategy.SELECTIN)

        # Check defaults
        expect(config.sql_only).to_be_false()
        expect(config.deferred_columns).to_equal([])
        expect(config.innerjoin).to_be_false()
        expect(config.columns).to_be_none()


class TestLazyFunction:
    """Test lazy() factory function."""

    def test_lazy_returns_loading_config(self):
        """Test lazy() returns LoadingConfig instance."""
        config = lazy()

        expect(isinstance(config, LoadingConfig)).to_be_true()

    def test_lazy_strategy(self):
        """Test lazy() sets LAZY strategy."""
        config = lazy()

        expect(config.strategy).to_equal(LoadingStrategy.LAZY)

    def test_lazy_defaults(self):
        """Test lazy() uses default values."""
        config = lazy()

        expect(config.sql_only).to_be_false()
        expect(config.innerjoin).to_be_false()
        expect(config.deferred_columns).to_equal([])


class TestJoinedFunction:
    """Test joined() factory function."""

    def test_joined_returns_loading_config(self):
        """Test joined() returns LoadingConfig instance."""
        config = joined()

        expect(isinstance(config, LoadingConfig)).to_be_true()

    def test_joined_strategy(self):
        """Test joined() sets JOINED strategy."""
        config = joined()

        expect(config.strategy).to_equal(LoadingStrategy.JOINED)

    def test_joined_default_innerjoin(self):
        """Test joined() uses LEFT JOIN by default."""
        config = joined()

        expect(config.innerjoin).to_be_false()

    def test_joined_with_innerjoin_true(self):
        """Test joined(innerjoin=True) sets INNER JOIN."""
        config = joined(innerjoin=True)

        expect(config.innerjoin).to_be_true()

    def test_joined_with_innerjoin_false(self):
        """Test joined(innerjoin=False) explicitly sets LEFT JOIN."""
        config = joined(innerjoin=False)

        expect(config.innerjoin).to_be_false()


class TestSubqueryFunction:
    """Test subquery() factory function."""

    def test_subquery_returns_loading_config(self):
        """Test subquery() returns LoadingConfig instance."""
        config = subquery()

        expect(isinstance(config, LoadingConfig)).to_be_true()

    def test_subquery_strategy(self):
        """Test subquery() sets SUBQUERY strategy."""
        config = subquery()

        expect(config.strategy).to_equal(LoadingStrategy.SUBQUERY)


class TestSelectinloadFunction:
    """Test selectinload() factory function."""

    def test_selectinload_returns_loading_config(self):
        """Test selectinload() returns LoadingConfig instance."""
        config = selectinload()

        expect(isinstance(config, LoadingConfig)).to_be_true()

    def test_selectinload_strategy(self):
        """Test selectinload() sets SELECTIN strategy."""
        config = selectinload()

        expect(config.strategy).to_equal(LoadingStrategy.SELECTIN)


class TestNoloadFunction:
    """Test noload() factory function."""

    def test_noload_returns_loading_config(self):
        """Test noload() returns LoadingConfig instance."""
        config = noload()

        expect(isinstance(config, LoadingConfig)).to_be_true()

    def test_noload_strategy(self):
        """Test noload() sets NOLOAD strategy."""
        config = noload()

        expect(config.strategy).to_equal(LoadingStrategy.NOLOAD)


class TestRaiseloadFunction:
    """Test raiseload() factory function."""

    def test_raiseload_returns_loading_config(self):
        """Test raiseload() returns LoadingConfig instance."""
        config = raiseload()

        expect(isinstance(config, LoadingConfig)).to_be_true()

    def test_raiseload_default_strategy(self):
        """Test raiseload() sets RAISE strategy by default."""
        config = raiseload()

        expect(config.strategy).to_equal(LoadingStrategy.RAISE)
        expect(config.sql_only).to_be_false()

    def test_raiseload_with_sql_only_true(self):
        """Test raiseload(sql_only=True) sets RAISE_ON_SQL strategy."""
        config = raiseload(sql_only=True)

        expect(config.strategy).to_equal(LoadingStrategy.RAISE_ON_SQL)
        expect(config.sql_only).to_be_true()

    def test_raiseload_with_sql_only_false(self):
        """Test raiseload(sql_only=False) sets RAISE strategy."""
        config = raiseload(sql_only=False)

        expect(config.strategy).to_equal(LoadingStrategy.RAISE)
        expect(config.sql_only).to_be_false()


class TestDeferFunction:
    """Test defer() factory function."""

    def test_defer_returns_loading_config(self):
        """Test defer() returns LoadingConfig instance."""
        config = defer("content")

        expect(isinstance(config, LoadingConfig)).to_be_true()

    def test_defer_single_column(self):
        """Test defer() with single column."""
        config = defer("content")

        expect(config.strategy).to_equal(LoadingStrategy.LAZY)
        expect(config.deferred_columns).to_equal(["content"])
        expect(config.columns).to_equal(["content"])

    def test_defer_multiple_columns(self):
        """Test defer() with multiple columns."""
        config = defer("content", "metadata", "summary")

        expect(config.deferred_columns).to_equal(["content", "metadata", "summary"])
        expect(config.columns).to_equal(["content", "metadata", "summary"])

    def test_defer_empty(self):
        """Test defer() with no columns."""
        config = defer()

        expect(config.deferred_columns).to_equal([])
        expect(config.columns).to_equal([])


class TestUndeferFunction:
    """Test undefer() factory function."""

    def test_undefer_returns_loading_config(self):
        """Test undefer() returns LoadingConfig instance."""
        config = undefer("content")

        expect(isinstance(config, LoadingConfig)).to_be_true()

    def test_undefer_single_column(self):
        """Test undefer() with single column."""
        config = undefer("content")

        expect(config.strategy).to_equal(LoadingStrategy.LAZY)
        expect(config.deferred_columns).to_equal([])
        expect(config.columns).to_equal(["content"])

    def test_undefer_multiple_columns(self):
        """Test undefer() with multiple columns."""
        config = undefer("content", "metadata", "summary")

        expect(config.deferred_columns).to_equal([])
        expect(config.columns).to_equal(["content", "metadata", "summary"])

    def test_undefer_empty(self):
        """Test undefer() with no columns."""
        config = undefer()

        expect(config.deferred_columns).to_equal([])
        expect(config.columns).to_equal([])


class TestLazyLoadingProxy:
    """Test LazyLoadingProxy class."""

    def test_proxy_creation(self):
        """Test LazyLoadingProxy can be created."""
        loader = RelationshipLoader()
        instance = Mock()
        config = lazy()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        expect(proxy._loader).to_equal(loader)
        expect(proxy._instance).to_equal(instance)
        expect(proxy._relationship).to_equal("posts")
        expect(proxy._config).to_equal(config)
        expect(proxy._is_loaded).to_be_false()
        expect(proxy._value).to_be_none()

    def test_proxy_is_loaded_property(self):
        """Test is_loaded property."""
        loader = RelationshipLoader()
        instance = Mock()
        config = lazy()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        expect(proxy.is_loaded).to_be_false()

        # Simulate loading
        proxy._is_loaded = True
        expect(proxy.is_loaded).to_be_true()

    @pytest.mark.asyncio
    async def test_proxy_fetch_raises_lazy_load_error(self):
        """Test fetch() raises LazyLoadError for RAISE strategy."""
        loader = RelationshipLoader()
        instance = Mock()
        config = raiseload()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        expect(lambda: await proxy.fetch()).to_raise(LazyLoadError)

    @pytest.mark.asyncio
    async def test_proxy_fetch_raises_sql_generation_error(self):
        """Test fetch() raises SQLGenerationError for RAISE_ON_SQL when not loaded."""
        loader = RelationshipLoader()
        instance = Mock()
        config = raiseload(sql_only=True)

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        expect(lambda: await proxy.fetch()).to_raise(SQLGenerationError)

    @pytest.mark.asyncio
    async def test_proxy_fetch_returns_cached_value(self):
        """Test fetch() returns cached value if already loaded."""
        loader = RelationshipLoader()
        instance = Mock()
        config = lazy()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        # Simulate loading
        proxy._is_loaded = True
        proxy._value = ["post1", "post2"]

        result = await proxy.fetch()

        expect(result).to_equal(["post1", "post2"])

    @pytest.mark.asyncio
    async def test_proxy_fetch_noload_returns_none(self):
        """Test fetch() returns None for NOLOAD strategy."""
        loader = RelationshipLoader()
        instance = Mock()
        config = noload()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        result = await proxy.fetch()

        expect(result).to_be_none()
        expect(proxy.is_loaded).to_be_true()

    @pytest.mark.asyncio
    async def test_proxy_fetch_lazy_loads_relationship(self):
        """Test fetch() loads relationship with LAZY strategy."""
        loader = RelationshipLoader()
        loader.load_lazy = AsyncMock(return_value=["post1", "post2"])
        instance = Mock()
        config = lazy()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        result = await proxy.fetch()

        loader.load_lazy.assert_called_once_with(instance, "posts", config)
        expect(result).to_equal(["post1", "post2"])
        expect(proxy.is_loaded).to_be_true()

    @pytest.mark.asyncio
    async def test_proxy_fetch_subquery_loads_relationship(self):
        """Test fetch() loads relationship with SUBQUERY strategy."""
        loader = RelationshipLoader()
        loader.load_subquery = AsyncMock(return_value=["post1", "post2"])
        instance = Mock()
        config = subquery()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        result = await proxy.fetch()

        loader.load_subquery.assert_called_once_with(instance, "posts", config)
        expect(result).to_equal(["post1", "post2"])
        expect(proxy.is_loaded).to_be_true()

    @pytest.mark.asyncio
    async def test_proxy_fetch_fallback_to_lazy(self):
        """Test fetch() falls back to lazy loading for JOINED/SELECTIN."""
        loader = RelationshipLoader()
        loader.load_lazy = AsyncMock(return_value=["post1"])
        instance = Mock()
        config = joined()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        result = await proxy.fetch()

        loader.load_lazy.assert_called_once()
        expect(result).to_equal(["post1"])

    @pytest.mark.asyncio
    async def test_proxy_await_syntax(self):
        """Test __await__ enables await syntax."""
        loader = RelationshipLoader()
        loader.load_lazy = AsyncMock(return_value=["post1"])
        instance = Mock()
        config = lazy()

        proxy = LazyLoadingProxy(loader, instance, "posts", config)

        # Should be able to use await directly
        result = await proxy

        expect(result).to_equal(["post1"])


class TestDeferredColumn:
    """Test DeferredColumn class."""

    def test_deferred_column_creation(self):
        """Test DeferredColumn can be created."""
        instance = Mock()
        config = defer("content")

        deferred = DeferredColumn(instance, "content", config)

        expect(deferred._instance).to_equal(instance)
        expect(deferred._column_name).to_equal("content")
        expect(deferred._config).to_equal(config)
        expect(deferred._is_loaded).to_be_false()
        expect(deferred._value).to_be_none()

    def test_deferred_column_is_loaded_property(self):
        """Test is_loaded property."""
        instance = Mock()
        config = defer("content")

        deferred = DeferredColumn(instance, "content", config)

        expect(deferred.is_loaded).to_be_false()

        # Simulate loading
        deferred._is_loaded = True
        expect(deferred.is_loaded).to_be_true()

    @pytest.mark.asyncio
    async def test_deferred_column_load_raises_lazy_load_error(self):
        """Test load() raises LazyLoadError for RAISE strategy."""
        instance = Mock()
        config = LoadingConfig(strategy=LoadingStrategy.RAISE)

        deferred = DeferredColumn(instance, "content", config)

        expect(lambda: await deferred.load()).to_raise(LazyLoadError)

    @pytest.mark.asyncio
    async def test_deferred_column_load_returns_cached_value(self):
        """Test load() returns cached value if already loaded."""
        instance = Mock()
        config = defer("content")

        deferred = DeferredColumn(instance, "content", config)

        # Simulate loading
        deferred._is_loaded = True
        deferred._value = "cached content"

        result = await deferred.load()

        expect(result).to_equal("cached content")

    @pytest.mark.asyncio
    async def test_deferred_column_load_without_table_name(self):
        """Test load() raises RuntimeError without table name."""
        instance = Mock(spec=[])  # No _table_name attribute
        config = defer("content")

        deferred = DeferredColumn(instance, "content", config)

        expect(lambda: await deferred.load()).to_raise(RuntimeError)

    @pytest.mark.asyncio
    async def test_deferred_column_load_without_pk(self):
        """Test load() raises RuntimeError without primary key."""
        instance = Mock()
        instance._table_name = "posts"
        instance._data = {}
        del instance.id  # Remove id attribute

        config = defer("content")
        deferred = DeferredColumn(instance, "content", config)

        expect(lambda: await deferred.load()).to_raise(RuntimeError)

    @pytest.mark.asyncio
    async def test_deferred_column_load_executes_query(self):
        """Test load() executes SQL query and loads value."""
        from ouroboros.postgres import connection

        instance = Mock()
        instance._table_name = "posts"
        instance._data = {"id": 1}

        config = defer("content")
        deferred = DeferredColumn(instance, "content", config)

        mock_execute = AsyncMock(return_value=[{"content": "loaded content"}])

        with patch.object(connection, 'execute', mock_execute):
            result = await deferred.load()

            # Check query was executed
            mock_execute.assert_called_once()
            args = mock_execute.call_args[0]
            expect("SELECT content FROM posts WHERE id = $1" in args[0]).to_be_true()
            expect(args[1]).to_equal([1])

            expect(result).to_equal("loaded content")
            expect(deferred.is_loaded).to_be_true()

    @pytest.mark.asyncio
    async def test_deferred_column_load_handles_empty_result(self):
        """Test load() handles empty query result."""
        from ouroboros.postgres import connection

        instance = Mock()
        instance._table_name = "posts"
        instance._data = {"id": 999}

        config = defer("content")
        deferred = DeferredColumn(instance, "content", config)

        mock_execute = AsyncMock(return_value=[])

        with patch.object(connection, 'execute', mock_execute):
            result = await deferred.load()

            expect(result).to_be_none()
            expect(deferred.is_loaded).to_be_true()

    @pytest.mark.asyncio
    async def test_deferred_column_await_syntax(self):
        """Test __await__ enables await syntax."""
        from ouroboros.postgres import connection

        instance = Mock()
        instance._table_name = "posts"
        instance._data = {"id": 1}

        config = defer("content")
        deferred = DeferredColumn(instance, "content", config)

        mock_execute = AsyncMock(return_value=[{"content": "test"}])

        with patch.object(connection, 'execute', mock_execute):
            # Should be able to use await directly
            result = await deferred

            expect(result).to_equal("test")


class TestRelationshipLoader:
    """Test RelationshipLoader class."""

    def test_relationship_loader_creation(self):
        """Test RelationshipLoader can be instantiated."""
        loader = RelationshipLoader()

        expect(loader is not None).to_be_true()

    @pytest.mark.asyncio
    async def test_load_lazy_returns_none_for_unknown_relationship(self):
        """Test load_lazy returns None for non-existent relationship."""
        loader = RelationshipLoader()
        instance = Mock()
        type(instance).__name__ = "User"

        # Mock the type() call to return a class without the relationship
        mock_class = type('MockUser', (), {})

        with patch('builtins.type', return_value=mock_class):
            result = await loader.load_lazy(instance, "posts", lazy())

        expect(result).to_be_none()

    @pytest.mark.asyncio
    async def test_load_lazy_handles_foreign_key_proxy(self):
        """Test load_lazy handles ForeignKeyProxy relationships."""
        loader = RelationshipLoader()

        # Create mock proxy
        mock_proxy = Mock()
        mock_proxy.fetch = AsyncMock(return_value="loaded_value")

        # Create instance with ForeignKeyProxy
        instance = Mock()
        instance.user = mock_proxy

        # Create mock class with ForeignKeyProxy descriptor
        from ouroboros.postgres.columns import ForeignKeyProxy
        mock_fk_proxy = Mock(spec=ForeignKeyProxy)
        mock_class = type('MockPost', (), {'user': mock_fk_proxy})

        with patch('builtins.type', return_value=mock_class):
            result = await loader.load_lazy(instance, "user", lazy())

        expect(result).to_equal("loaded_value")

    @pytest.mark.asyncio
    async def test_load_lazy_handles_back_reference(self):
        """Test load_lazy handles BackReference relationships."""
        loader = RelationshipLoader()

        # Create mock query object
        mock_query = Mock()
        mock_query.fetch_all = AsyncMock(return_value=["post1", "post2"])

        # Create instance with BackReference query
        instance = Mock()
        instance.posts = mock_query

        # Create mock class with BackReference descriptor
        from ouroboros.postgres.columns import BackReference
        mock_back_ref = Mock(spec=BackReference)
        mock_class = type('MockUser', (), {'posts': mock_back_ref})

        with patch('builtins.type', return_value=mock_class):
            result = await loader.load_lazy(instance, "posts", lazy())

        expect(result).to_equal(["post1", "post2"])

    def test_load_joined_returns_query(self):
        """Test load_joined returns the query unchanged (placeholder)."""
        loader = RelationshipLoader()
        mock_query = Mock()

        result = loader.load_joined(mock_query, ["posts", "profile"], joined())

        # Currently returns query unchanged
        expect(result).to_equal(mock_query)

    @pytest.mark.asyncio
    async def test_load_selectin_empty_instances(self):
        """Test load_selectin returns empty dict for empty instances."""
        loader = RelationshipLoader()

        result = await loader.load_selectin([], "posts", selectinload())

        expect(result).to_equal({})

    @pytest.mark.asyncio
    async def test_load_selectin_unknown_relationship(self):
        """Test load_selectin returns empty dict for unknown relationship."""
        loader = RelationshipLoader()
        instance = Mock()
        mock_class = type('MockUser', (), {})

        with patch('builtins.type', return_value=mock_class):
            result = await loader.load_selectin([instance], "posts", selectinload())

        expect(result).to_equal({})

    @pytest.mark.asyncio
    async def test_load_selectin_back_reference(self):
        """Test load_selectin batch loads BackReference relationships."""
        # Import execute function
        from ouroboros.postgres import connection

        loader = RelationshipLoader()

        # Create mock BackReference
        from ouroboros.postgres.columns import BackReference
        mock_back_ref = Mock(spec=BackReference)
        mock_back_ref.source_table = "posts"
        mock_back_ref.source_column = "user_id"
        mock_back_ref.target_column = "id"

        # Create instances
        instance1 = Mock()
        instance1._data = {"id": 1}
        instance2 = Mock()
        instance2._data = {"id": 2}

        mock_class = type('MockUser', (), {'posts': mock_back_ref})

        # Patch execute on the connection module
        original_execute = getattr(connection, 'execute', None)
        mock_execute = AsyncMock(return_value=[
            {"id": 10, "user_id": 1, "title": "Post 1"},
            {"id": 11, "user_id": 1, "title": "Post 2"},
            {"id": 12, "user_id": 2, "title": "Post 3"},
        ])

        with patch('builtins.type', return_value=mock_class):
            with patch.object(connection, 'execute', mock_execute):
                result = await loader.load_selectin([instance1, instance2], "posts", selectinload())

                # Check query was executed with IN clause
                mock_execute.assert_called_once()
                args = mock_execute.call_args[0]
                expect("SELECT * FROM posts" in args[0]).to_be_true()
                expect("WHERE user_id IN" in args[0]).to_be_true()
                expect(args[1]).to_equal([1, 2])

                # Check results are grouped by foreign key
                expect(result[1]).to_equal([
                    {"id": 10, "user_id": 1, "title": "Post 1"},
                    {"id": 11, "user_id": 1, "title": "Post 2"},
                ])
                expect(result[2]).to_equal([
                    {"id": 12, "user_id": 2, "title": "Post 3"},
                ])

    @pytest.mark.asyncio
    async def test_load_selectin_skips_instances_without_pk(self):
        """Test load_selectin skips instances without primary key."""
        from ouroboros.postgres import connection

        loader = RelationshipLoader()

        from ouroboros.postgres.columns import BackReference
        mock_back_ref = Mock(spec=BackReference)
        mock_back_ref.source_table = "posts"
        mock_back_ref.source_column = "user_id"
        mock_back_ref.target_column = "id"

        instance1 = Mock()
        instance1._data = {}  # No id
        instance1.id = None  # Explicitly set id to None

        instance2 = Mock()
        instance2._data = {"id": 2}
        instance2.id = 2  # Explicitly set id

        mock_class = type('MockUser', (), {'posts': mock_back_ref})

        mock_execute = AsyncMock(return_value=[
            {"id": 12, "user_id": 2, "title": "Post 3"},
        ])

        with patch('builtins.type', return_value=mock_class):
            with patch.object(connection, 'execute', mock_execute):
                result = await loader.load_selectin([instance1, instance2], "posts", selectinload())

                # Should only query for instance2
                args = mock_execute.call_args[0]
                expect(args[1]).to_equal([2])

    @pytest.mark.asyncio
    async def test_load_subquery_delegates_to_lazy(self):
        """Test load_subquery delegates to load_lazy (placeholder)."""
        loader = RelationshipLoader()
        loader.load_lazy = AsyncMock(return_value=["post1"])

        instance = Mock()
        config = subquery()

        result = await loader.load_subquery(instance, "posts", config)

        loader.load_lazy.assert_called_once_with(instance, "posts", config)
        expect(result).to_equal(["post1"])


class TestCustomExceptions:
    """Test custom exception classes."""

    def test_lazy_load_error_inherits_runtime_error(self):
        """Test LazyLoadError inherits from RuntimeError."""
        error = LazyLoadError("test message")

        expect(isinstance(error, RuntimeError)).to_be_true()
        expect(str(error)).to_equal("test message")

    def test_lazy_load_error_can_be_raised(self):
        """Test LazyLoadError can be raised and caught."""
        expect(lambda: raise LazyLoadError("test error")).to_raise(LazyLoadError)

    def test_sql_generation_error_inherits_runtime_error(self):
        """Test SQLGenerationError inherits from RuntimeError."""
        error = SQLGenerationError("test message")

        expect(isinstance(error, RuntimeError)).to_be_true()
        expect(str(error)).to_equal("test message")

    def test_sql_generation_error_can_be_raised(self):
        """Test SQLGenerationError can be raised and caught."""
        expect(lambda: raise SQLGenerationError("SQL not allowed")).to_raise(SQLGenerationError)

    def test_exceptions_are_distinct(self):
        """Test exceptions are different classes."""
        lazy_error = LazyLoadError("lazy")
        sql_error = SQLGenerationError("sql")

        expect(type(lazy_error) is not type(sql_error)).to_be_true()

    def test_exceptions_can_be_caught_separately(self):
        """Test exceptions can be caught separately."""
        # Catch LazyLoadError
        try:
            raise LazyLoadError("lazy")
        except LazyLoadError as e:
            expect(str(e)).to_equal("lazy")
        except SQLGenerationError:
            pytest.fail("Should not catch SQLGenerationError")

        # Catch SQLGenerationError
        try:
            raise SQLGenerationError("sql")
        except LazyLoadError:
            pytest.fail("Should not catch LazyLoadError")
        except SQLGenerationError as e:
            expect(str(e)).to_equal("sql")

    def test_exceptions_can_be_caught_as_runtime_error(self):
        """Test both exceptions can be caught as RuntimeError."""
        # LazyLoadError
        try:
            raise LazyLoadError("lazy")
        except RuntimeError as e:
            expect(isinstance(e, LazyLoadError)).to_be_true()

        # SQLGenerationError
        try:
            raise SQLGenerationError("sql")
        except RuntimeError as e:
            expect(isinstance(e, SQLGenerationError)).to_be_true()
