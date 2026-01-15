"""
Entry point for running ouroboros.qc as a module.

Usage:
    python -m ouroboros.qc [args]
"""

from .cli import main

if __name__ == "__main__":
    exit(main())
