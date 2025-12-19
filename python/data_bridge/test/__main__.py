"""
Entry point for running data_bridge.test as a module.

Usage:
    python -m data_bridge.test [args]
"""

from .cli import main

if __name__ == "__main__":
    exit(main())
