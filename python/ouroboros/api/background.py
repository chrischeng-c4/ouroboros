"""
Background tasks that run after the response is sent.

Similar to FastAPI's BackgroundTasks but simplified for data-bridge-api.
Allows queueing functions (sync or async) to execute after the HTTP response
is sent to the client.

Example:
    @app.post("/send-email")
    async def send_email(
        email: str,
        background: Annotated[BackgroundTasks, Depends()]
    ):
        background.add_task(send_email_async, email, subject="Welcome")
        return {"message": "Email queued"}
"""
from typing import Callable, Any, List, Tuple
import asyncio
import logging

logger = logging.getLogger(__name__)


class BackgroundTasks:
    """
    A collection of background tasks to run after the response is sent.

    Tasks are executed in order after the HTTP response is sent to the client.
    If a task raises an exception, it is logged but does not affect other tasks.

    Usage:
        background = BackgroundTasks()
        background.add_task(send_notification, user_id=123)
        background.add_task(log_analytics, event="signup")
        await background.run()  # Execute all tasks

    Attributes:
        _tasks: List of (function, args, kwargs) tuples to execute
    """

    def __init__(self):
        """Initialize an empty collection of background tasks."""
        self._tasks: List[Tuple[Callable, tuple, dict]] = []

    def add_task(
        self,
        func: Callable,
        *args: Any,
        **kwargs: Any
    ) -> None:
        """
        Add a background task to be executed after the response.

        The function can be either synchronous or asynchronous. Synchronous
        functions will be executed in a thread pool to avoid blocking the
        event loop.

        Args:
            func: The function to call (can be sync or async)
            *args: Positional arguments for the function
            **kwargs: Keyword arguments for the function

        Example:
            background.add_task(cleanup_temp_files, "/tmp/upload")
            background.add_task(send_email, "user@example.com", subject="Welcome")
        """
        self._tasks.append((func, args, kwargs))

    async def run(self) -> None:
        """
        Execute all background tasks in order.

        Tasks are executed sequentially in the order they were added.
        If a task raises an exception, it is logged but does not prevent
        other tasks from running.

        Async tasks are awaited directly. Sync tasks are executed in a
        thread pool via `loop.run_in_executor()` to avoid blocking.

        Example:
            background = BackgroundTasks()
            background.add_task(my_task)
            await background.run()  # Executes all tasks
        """
        for func, args, kwargs in self._tasks:
            try:
                if asyncio.iscoroutinefunction(func):
                    # Async function - await directly
                    await func(*args, **kwargs)
                else:
                    # Sync function - run in thread pool to avoid blocking
                    loop = asyncio.get_event_loop()
                    await loop.run_in_executor(None, lambda: func(*args, **kwargs))
            except Exception as e:
                # Log but don't raise - background tasks shouldn't crash the app
                logger.exception(
                    f"Background task {func.__name__} failed: {e}",
                    exc_info=True
                )

    def __len__(self) -> int:
        """
        Return the number of queued tasks.

        Returns:
            Number of tasks in the queue

        Example:
            background = BackgroundTasks()
            background.add_task(my_task)
            assert len(background) == 1
        """
        return len(self._tasks)

    def clear(self) -> None:
        """
        Clear all pending tasks.

        Removes all tasks from the queue without executing them.
        Useful for cleanup or testing scenarios.

        Example:
            background = BackgroundTasks()
            background.add_task(my_task)
            background.clear()
            assert len(background) == 0
        """
        self._tasks.clear()

    def __repr__(self) -> str:
        """Return string representation of BackgroundTasks."""
        return f"BackgroundTasks(tasks={len(self._tasks)})"


def get_background_tasks() -> BackgroundTasks:
    """
    Factory function for BackgroundTasks dependency.

    Use this with Depends() to inject a BackgroundTasks instance
    into your handler function.

    Returns:
        A new BackgroundTasks instance (request-scoped)

    Example:
        @app.post("/process")
        async def process_data(
            background: Annotated[BackgroundTasks, Depends(get_background_tasks)]
        ):
            background.add_task(heavy_processing)
            return {"status": "processing"}
    """
    return BackgroundTasks()
