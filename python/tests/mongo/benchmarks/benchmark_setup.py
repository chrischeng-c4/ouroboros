"""
Automatic setup for MongoDB benchmarks.

This module initializes MongoDB connections when imported, allowing benchmarks
to be auto-discovered and run by dbtest without explicit setup.

Import this module at the top of benchmark files to enable auto-initialization.
"""

import asyncio
import os
from typing import Optional

_setup_complete = False
_motor_client = None


def get_mongodb_uri() -> str:
    """Get MongoDB URI from environment or use default."""
    return os.environ.get(
        "MONGODB_URI",
        "mongodb://localhost:27017/data-bridge-benchmark"
    )


async def _async_setup():
    """Initialize MongoDB connections asynchronously."""
    global _setup_complete, _motor_client

    if _setup_complete:
        return

    from ouroboros import init, close, is_connected
    from beanie import init_beanie
    from motor.motor_asyncio import AsyncIOMotorClient
    from .models import BEANIE_MODELS

    mongodb_uri = get_mongodb_uri()
    
    # Extract database name from URI if possible
    # e.g., mongodb://localhost:27017/my_db -> my_db
    import urllib.parse
    parsed = urllib.parse.urlparse(mongodb_uri)
    db_name = parsed.path.strip("/") or "data-bridge-benchmark"

    # Initialize data-bridge
    if is_connected():
        await close()
    await init(mongodb_uri)

    # Initialize Beanie
    _motor_client = AsyncIOMotorClient(mongodb_uri)
    await init_beanie(
        database=_motor_client[db_name],
        document_models=BEANIE_MODELS,
    )

    _setup_complete = True


def ensure_setup():
    """
    Ensure MongoDB is initialized (synchronous wrapper).

    This is called automatically when benchmarks are imported.
    """
    global _setup_complete

    if not _setup_complete:
        # Run async setup in a new event loop if needed
        try:
            loop = asyncio.get_running_loop()
            # If we're in an async context, schedule setup
            # Note: This won't block, setup will happen before benchmarks run
        except RuntimeError:
            # No event loop running, create one
            asyncio.run(_async_setup())


async def async_ensure_setup():
    """
    Ensure MongoDB is initialized (async version).

    Call this from benchmark functions if needed.
    """
    await _async_setup()


async def cleanup():
    """Clean up MongoDB connections."""
    global _motor_client, _setup_complete

    if not _setup_complete:
        return

    from ouroboros import close

    await close()
    if _motor_client:
        _motor_client.close()
        _motor_client = None

    _setup_complete = False


# Auto-initialize when this module is imported (for dbtest auto-discovery)
# This ensures MongoDB is ready when benchmarks are loaded
try:
    ensure_setup()
except Exception as e:
    # If setup fails during import, benchmarks will fail with clear error
    print(f"Warning: Benchmark setup failed: {e}")
    print("MongoDB connection will be attempted when benchmarks run")
