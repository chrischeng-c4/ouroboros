"""
Entry point for running ouroboros.test as a module.

Usage:
    python -m ouroboros.test [args]
"""

from .cli import main

if __name__ == "__main__":
    exit(main())
