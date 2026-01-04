"""
Dependency Injection System for data-bridge API

Provides FastAPI-compatible dependency injection with:
- Function dependencies: Depends(get_db)
- Class dependencies: Depends(Database)
- Scoped caching: transient, request, singleton
- Sub-dependencies: Automatic resolution
- Async support: Works with async and sync functions

Example:
    async def get_db():
        return Database()

    async def get_current_user(db: Annotated[Database, Depends(get_db)]):
        return await db.get_user()

    @app.get("/me")
    async def get_me(user: Annotated[User, Depends(get_current_user)]):
        return user
"""

from typing import (
    Any, Callable, Dict, Generic, List, Optional, Type, TypeVar,
    Union, get_type_hints, Annotated, get_origin, get_args
)
import asyncio
import inspect
from dataclasses import dataclass, field
from enum import Enum
from functools import wraps

T = TypeVar('T')


class Scope(str, Enum):
    """Dependency scope."""
    TRANSIENT = "transient"  # New instance every call
    REQUEST = "request"      # Cached per request
    SINGLETON = "singleton"  # Cached for app lifetime


@dataclass
class Depends:
    """Declare a dependency.

    Example:
        async def get_db():
            return Database()

        @app.get("/users")
        async def list_users(
            db: Annotated[Database, Depends(get_db)]
        ) -> List[User]:
            return await db.get_users()

    Args:
        dependency: A callable (function or class) that provides the dependency
        use_cache: Whether to cache the result (default: True)
        scope: Cache scope - transient, request, or singleton
    """
    dependency: Optional[Callable[..., Any]] = None
    use_cache: bool = True
    scope: Scope = Scope.REQUEST

    def __init__(
        self,
        dependency: Optional[Callable[..., Any]] = None,
        *,
        use_cache: bool = True,
        scope: Union[Scope, str] = Scope.REQUEST,
    ):
        self.dependency = dependency
        self.use_cache = use_cache
        self.scope = Scope(scope) if isinstance(scope, str) else scope

    def __repr__(self) -> str:
        dep_name = getattr(self.dependency, '__name__', str(self.dependency))
        return f"Depends({dep_name}, scope={self.scope.value})"


class DependencyNode:
    """Node in the dependency graph."""

    def __init__(
        self,
        name: str,
        factory: Callable[..., Any],
        scope: Scope,
        sub_dependencies: List[str],
        param_mapping: Optional[Dict[str, str]] = None,
    ):
        self.name = name
        self.factory = factory
        self.scope = scope
        self.sub_dependencies = sub_dependencies
        self.param_mapping = param_mapping or {}  # Maps dependency key -> param name
        self.is_async = asyncio.iscoroutinefunction(factory)
        self.is_generator = inspect.isgeneratorfunction(factory)
        self.is_async_generator = inspect.isasyncgenfunction(factory)


class RequestContext:
    """Context for a single request, holding cached dependencies."""

    def __init__(self):
        self._cache: Dict[str, Any] = {}
        self._generators: List[Any] = []  # For cleanup
        self._async_generators: List[Any] = []

    def get(self, name: str) -> Optional[Any]:
        return self._cache.get(name)

    def set(self, name: str, value: Any) -> None:
        self._cache[name] = value

    def add_generator(self, gen: Any) -> None:
        self._generators.append(gen)

    def add_async_generator(self, gen: Any) -> None:
        self._async_generators.append(gen)

    async def cleanup(self) -> None:
        """Cleanup generators (for context managers)."""
        # Cleanup sync generators
        for gen in reversed(self._generators):
            try:
                next(gen, None)
            except StopIteration:
                pass
            except Exception:
                pass

        # Cleanup async generators
        for gen in reversed(self._async_generators):
            try:
                await gen.asend(None)
            except StopAsyncIteration:
                pass
            except Exception:
                pass

        self._cache.clear()
        self._generators.clear()
        self._async_generators.clear()


class DependencyContainer:
    """Container for managing dependencies."""

    def __init__(self):
        self._nodes: Dict[str, DependencyNode] = {}
        self._singletons: Dict[str, Any] = {}
        self._resolution_order: List[str] = []
        self._compiled: bool = False

    def register(
        self,
        name: str,
        factory: Callable[..., Any],
        scope: Scope = Scope.REQUEST,
        sub_dependencies: Optional[List[str]] = None,
        param_mapping: Optional[Dict[str, str]] = None,
    ) -> None:
        """Register a dependency."""
        if self._compiled:
            raise RuntimeError("Cannot register after compilation")

        self._nodes[name] = DependencyNode(
            name=name,
            factory=factory,
            scope=scope,
            sub_dependencies=sub_dependencies or [],
            param_mapping=param_mapping or {},
        )

    def compile(self) -> None:
        """Compile the container, computing resolution order."""
        if self._compiled:
            return

        # Validate dependencies exist
        for node in self._nodes.values():
            for dep in node.sub_dependencies:
                if dep not in self._nodes:
                    raise ValueError(
                        f"Dependency '{dep}' required by '{node.name}' is not registered"
                    )

        # Topological sort
        self._resolution_order = self._topological_sort()
        self._compiled = True

    def _topological_sort(self) -> List[str]:
        """Kahn's algorithm for topological sorting."""
        in_degree: Dict[str, int] = {name: 0 for name in self._nodes}
        graph: Dict[str, List[str]] = {name: [] for name in self._nodes}

        for name, node in self._nodes.items():
            for dep in node.sub_dependencies:
                graph[dep].append(name)
                in_degree[name] += 1

        # Start with nodes that have no dependencies
        queue = [name for name, degree in in_degree.items() if degree == 0]
        result: List[str] = []

        while queue:
            node = queue.pop(0)
            result.append(node)

            for neighbor in graph[node]:
                in_degree[neighbor] -= 1
                if in_degree[neighbor] == 0:
                    queue.append(neighbor)

        if len(result) != len(self._nodes):
            remaining = set(self._nodes.keys()) - set(result)
            raise ValueError(f"Circular dependency detected involving: {remaining}")

        return result

    def get_resolution_order(self, required: List[str]) -> List[str]:
        """Get resolution order for specific dependencies."""
        if not self._compiled:
            raise RuntimeError("Container must be compiled first")

        # Find all transitive dependencies
        needed: set = set()
        stack = list(required)

        while stack:
            name = stack.pop()
            if name in needed:
                continue
            needed.add(name)

            if name in self._nodes:
                stack.extend(self._nodes[name].sub_dependencies)

        # Return in topological order
        return [name for name in self._resolution_order if name in needed]

    async def resolve(
        self,
        name: str,
        context: RequestContext,
    ) -> Any:
        """Resolve a single dependency."""
        node = self._nodes.get(name)
        if node is None:
            raise ValueError(f"Unknown dependency: {name}")

        # Check singleton cache
        if node.scope == Scope.SINGLETON and name in self._singletons:
            return self._singletons[name]

        # Check request cache
        if node.scope == Scope.REQUEST:
            cached = context.get(name)
            if cached is not None:
                return cached

        # Resolve sub-dependencies first
        sub_values: Dict[str, Any] = {}
        for sub_name in node.sub_dependencies:
            resolved_value = await self.resolve(sub_name, context)
            # Use param mapping if available, otherwise use dependency name
            param_name = node.param_mapping.get(sub_name, sub_name)
            sub_values[param_name] = resolved_value

        # Call factory
        if node.is_async:
            value = await node.factory(**sub_values)
        elif node.is_generator:
            gen = node.factory(**sub_values)
            value = next(gen)
            context.add_generator(gen)
        elif node.is_async_generator:
            gen = node.factory(**sub_values)
            value = await gen.asend(None)
            context.add_async_generator(gen)
        else:
            value = node.factory(**sub_values)

        # Cache based on scope
        if node.scope == Scope.SINGLETON:
            self._singletons[name] = value
        elif node.scope == Scope.REQUEST:
            context.set(name, value)

        return value

    async def resolve_all(
        self,
        required: List[str],
        context: Optional[RequestContext] = None,
    ) -> Dict[str, Any]:
        """Resolve multiple dependencies in correct order."""
        if context is None:
            context = RequestContext()

        order = self.get_resolution_order(required)
        result: Dict[str, Any] = {}

        for name in order:
            result[name] = await self.resolve(name, context)

        return {name: result[name] for name in required if name in result}


def extract_dependencies(func: Callable) -> Dict[str, Depends]:
    """Extract dependencies from a function's type hints."""
    dependencies: Dict[str, Depends] = {}

    try:
        hints = get_type_hints(func, include_extras=True)
    except Exception:
        return dependencies

    sig = inspect.signature(func)

    for param_name, param in sig.parameters.items():
        if param_name in ('self', 'cls'):
            continue

        hint = hints.get(param_name)
        if hint is None:
            continue

        origin = get_origin(hint)
        if origin is Annotated:
            args = get_args(hint)
            for arg in args[1:]:
                if isinstance(arg, Depends):
                    dependencies[param_name] = arg
                    break

    return dependencies


def build_dependency_graph(
    func: Callable,
    container: DependencyContainer,
    prefix: str = "",
    _global_deps: Optional[Dict[int, str]] = None,
) -> List[str]:
    """Build dependency graph for a function, registering all dependencies.

    Args:
        func: The function to extract dependencies from
        container: The container to register dependencies in
        prefix: Prefix for dependency names
        _global_deps: Internal mapping of factory id -> registered name for deduplication
    """
    # Track global dependencies by factory identity for deduplication
    if _global_deps is None:
        _global_deps = {}

    dependencies = extract_dependencies(func)
    registered: List[str] = []

    for param_name, depends in dependencies.items():
        if depends.dependency is None:
            continue

        dep_factory = depends.dependency
        factory_id = id(dep_factory)

        # Check if this factory was already registered (shared dependency)
        if factory_id in _global_deps:
            # Reuse existing registration name
            existing_name = _global_deps[factory_id]
            registered.append(existing_name)
            continue

        dep_name = f"{prefix}{param_name}" if prefix else param_name

        # First, recursively process sub-dependencies to populate _global_deps
        sub_deps_list = build_dependency_graph(
            dep_factory,
            container,
            prefix="",  # Use no prefix for sub-deps to enable sharing
            _global_deps=_global_deps,
        )

        # Build param mapping: maps dependency key -> factory param name
        param_mapping: Dict[str, str] = {}
        sub_dependencies_of_factory = extract_dependencies(dep_factory)
        for sub_param_name, sub_depends in sub_dependencies_of_factory.items():
            if sub_depends.dependency is not None:
                sub_factory_id = id(sub_depends.dependency)
                # Find the registered name for this sub-dependency
                if sub_factory_id in _global_deps:
                    actual_dep_name = _global_deps[sub_factory_id]
                    param_mapping[actual_dep_name] = sub_param_name

        # Register this dependency
        container.register(
            name=dep_name,
            factory=dep_factory,
            scope=depends.scope,
            sub_dependencies=sub_deps_list,
            param_mapping=param_mapping,
        )

        # Track this factory for deduplication
        _global_deps[factory_id] = dep_name
        registered.append(dep_name)

    return registered
