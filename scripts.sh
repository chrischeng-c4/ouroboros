#!/bin/bash

# Quality control script for data-bridge project using uv

case "$1" in
    lint)
        echo "Running linters..."
        uv run ruff check src/ tests/
        uv run pylint src/data_bridge
        ;;
    format)
        echo "Formatting code..."
        uv run black src/ tests/
        uv run isort src/ tests/
        uv run ruff check --fix src/ tests/
        ;;
    test)
        echo "Running tests..."
        uv run pytest "${@:2}"
        ;;
    test-cov)
        echo "Running tests with coverage..."
        uv run pytest --cov=data_bridge --cov-report=term-missing --cov-report=html
        ;;
    typecheck)
        echo "Running type checking..."
        uv run mypy src/
        ;;
    qc)
        echo "Running all quality checks..."
        uv run ruff check src/ tests/ && \
        uv run mypy src/ && \
        uv run pytest
        echo "All quality checks passed!"
        ;;
    *)
        echo "Usage: ./scripts.sh {lint|format|test|test-cov|typecheck|qc}"
        echo ""
        echo "Commands:"
        echo "  lint       - Run ruff and pylint"
        echo "  format     - Format code with black, isort, and ruff"
        echo "  test       - Run pytest (pass additional args after test)"
        echo "  test-cov   - Run tests with coverage report"
        echo "  typecheck  - Run mypy type checking"
        echo "  qc         - Run all quality checks (lint, typecheck, test)"
        exit 1
        ;;
esac