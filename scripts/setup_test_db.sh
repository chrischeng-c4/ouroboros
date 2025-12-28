#!/bin/bash
# Setup PostgreSQL test database for integration tests
set -e

echo "Setting up PostgreSQL test database..."

# Check if Docker container is running
if ! docker ps | grep -q rstn-postgres; then
    echo "Error: PostgreSQL container 'rstn-postgres' is not running"
    exit 1
fi

# Drop existing test database if present
echo "Dropping existing test database (if present)..."
docker exec rstn-postgres psql -U rstn -c "DROP DATABASE IF EXISTS data_bridge_test;" || true

# Create fresh test database
echo "Creating test database 'data_bridge_test'..."
docker exec rstn-postgres psql -U rstn -c "CREATE DATABASE data_bridge_test;"

echo "Test database ready!"
echo "Connection URI: postgresql://rstn:rstn@localhost:5432/data_bridge_test"
