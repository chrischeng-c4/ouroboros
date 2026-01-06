# data-bridge-test Examples

This directory contains example code demonstrating how to use the data-bridge-test framework.

## Available Examples

### JUnit Reporter Example

**File**: `junit_reporter_example.rs`

Demonstrates how to use the JUnit XML reporter for CI/CD integration.

**Run it**:
```bash
cargo run --example junit_reporter_example -p data-bridge-test
```

**What it does**:
- Creates sample test results (passed, failed, error, skipped)
- Generates JUnit XML report (`test-results.xml`)
- Generates Markdown report (`test-report.md`)
- Shows how to use different reporter formats

**Output files**:
- `test-results.xml` - JUnit XML format for CI/CD systems
- `test-report.md` - Human-readable markdown report

**Use cases**:
- GitHub Actions test reporting
- GitLab CI test integration
- Jenkins JUnit plugin
- CircleCI test results
- Any CI/CD system that supports JUnit XML format

## Documentation

For detailed CI/CD integration guides, see:
- [JUnit Integration Guide](../docs/junit-integration.md)

## Need Help?

- Check the [main documentation](../README.md) for general usage
- See [../docs/junit-integration.md](../docs/junit-integration.md) for CI/CD setup
- Review the example source code for implementation patterns
