"""Type stubs for ouroboros.tasks module."""

from typing import Any, Awaitable, Callable, Optional, TypeVar, overload

# Type variable for task functions
F = TypeVar("F", bound=Callable[..., Any])

# Initialization
async def init(
    redis_url: str,
    *,
    broker_type: Optional[str] = None,
    nats_url: Optional[str] = None,
    pubsub_project_id: Optional[str] = None,
    pubsub_topic: Optional[str] = None,
    pubsub_subscription: Optional[str] = None,
) -> None:
    """
    Initialize the task queue system.

    Args:
        redis_url: Redis connection URL for result backend
        broker_type: Broker type - "nats" (default) or "pubsub"
        nats_url: NATS connection URL (used when broker_type is "nats")
        pubsub_project_id: GCP project ID (used when broker_type is "pubsub")
        pubsub_topic: Pub/Sub topic name (default: "tasks")
        pubsub_subscription: Pub/Sub subscription name (default: "task-worker")

    Examples:
        # NATS (default)
        await init(redis_url="redis://localhost:6379", nats_url="nats://localhost:4222")

        # Google Cloud Pub/Sub
        await init(
            redis_url="redis://localhost:6379",
            broker_type="pubsub",
            pubsub_project_id="my-project",
        )

        # From environment variables
        await init(redis_url="redis://localhost:6379")
        # Reads BROKER_TYPE, NATS_URL, PUBSUB_PROJECT_ID, etc.

    Raises:
        ValueError: If broker_type is unknown or required parameters are missing
        ConnectionError: If connection to Redis or broker fails
    """
    ...

# Task decorator
@overload
def task(func: F) -> Task: ...

@overload
def task(
    *,
    name: Optional[str] = None,
    queue: str = "default",
    max_retries: int = 3,
    retry_delay: float = 1.0,
) -> Callable[[F], Task]: ...

def task(
    func: Optional[F] = None,
    *,
    name: Optional[str] = None,
    queue: str = "default",
    max_retries: int = 3,
    retry_delay: float = 1.0,
) -> Task | Callable[[F], Task]:
    """
    Decorator to create a task.

    Args:
        func: Function to decorate (auto-filled by decorator syntax)
        name: Task name (defaults to function name)
        queue: Queue name (default: "default")
        max_retries: Maximum retry attempts (default: 3)
        retry_delay: Delay between retries in seconds (default: 1.0)

    Returns:
        Task instance that can execute the function asynchronously

    Example:
        @task(name="add", queue="math", max_retries=5)
        async def add(x: int, y: int) -> int:
            return x + y

        # Execute
        result = await add.delay(1, 2)
        print(await result.get())  # 3
    """
    ...

# Task class
class Task:
    """A task that can be executed asynchronously."""

    name: str
    queue: str

    def delay(self, *args: Any, **kwargs: Any) -> Awaitable[AsyncResult]:
        """Send task for async execution with positional args."""
        ...

    def apply_async(
        self,
        *args: Any,
        countdown: Optional[float] = None,
        eta: Optional[str] = None,
        **kwargs: Any,
    ) -> Awaitable[AsyncResult]:
        """Send task with options."""
        ...

    def s(self, *args: Any, **kwargs: Any) -> TaskSignature:
        """Create a signature for this task (for workflows)."""
        ...

# AsyncResult class
class AsyncResult:
    """Handle to track async task execution."""

    task_id: str

    def ready(self) -> Awaitable[bool]:
        """Check if task is complete."""
        ...

    def get(self, timeout: Optional[float] = None) -> Awaitable[Any]:
        """Get result (waits for completion)."""
        ...

    def state(self) -> Awaitable[str]:
        """Get current state without waiting."""
        ...

    def info(self) -> Awaitable[dict[str, Any]]:
        """Get full result object (includes state, result, error, timestamps)."""
        ...

# TaskSignature class
class TaskSignature:
    """Represents a task invocation that can be used in workflows."""

    task_name: str

    def apply_async(self) -> Awaitable[AsyncResult]:
        """Execute this signature."""
        ...

# Chain workflow
class Chain:
    """Executes tasks sequentially, passing output of each task as input to the next."""

    def __init__(self, tasks: list[TaskSignature]) -> None: ...
    def apply_async(self) -> Awaitable[AsyncResult]:
        """Execute the chain."""
        ...
    def __len__(self) -> int: ...

# Group workflow
class Group:
    """Executes tasks in parallel."""

    def __init__(self, tasks: list[TaskSignature]) -> None: ...
    def apply_async(self) -> Awaitable[GroupResult]:
        """Execute the group."""
        ...
    def __len__(self) -> int: ...

# GroupResult class
class GroupResult:
    """Result handle for a Group workflow."""

    task_ids: list[str]

    def get(self, timeout: Optional[float] = None) -> Awaitable[list[Any]]:
        """Get results for all tasks (waits for all to complete)."""
        ...

# Chord workflow
class Chord:
    """Executes tasks in parallel, then executes a callback with the results."""

    def __init__(self, header: list[TaskSignature], callback: TaskSignature) -> None: ...
    def apply_async(self) -> Awaitable[AsyncResult]:
        """Execute the chord."""
        ...

# Exports
__all__ = [
    "Task",
    "AsyncResult",
    "TaskSignature",
    "Chain",
    "Group",
    "GroupResult",
    "Chord",
    "task",
    "init",
]
