# JUnit XML Reporter - CI/CD Integration Guide

This guide shows how to integrate the data-bridge-test JUnit XML reporter with various CI/CD systems.

## Table of Contents

- [Quick Start](#quick-start)
- [GitHub Actions](#github-actions)
- [GitLab CI](#gitlab-ci)
- [Jenkins](#jenkins)
- [CircleCI](#circleci)
- [Example Output](#example-output)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Generate JUnit XML Report

```rust
use data_bridge_test::reporter::{Reporter, TestReport};
use data_bridge_test::runner::{TestMeta, TestResult};

// Create test results
let results = vec![
    TestResult::passed(TestMeta::new("test_example"), 100),
    // ... more test results
];

// Create report
let report = TestReport::new("MyTestSuite", results);

// Generate JUnit XML
let reporter = Reporter::junit();
let junit_xml = reporter.generate(&report);

// Write to file
std::fs::write("test-results.xml", junit_xml)?;
```

### Run the Example

```bash
# Run the included example
cargo run --example junit_reporter_example -p data-bridge-test

# This generates test-results.xml in the current directory
```

## GitHub Actions

### Basic Integration

Create `.github/workflows/test.yml`:

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run tests and generate JUnit XML
        run: |
          cargo run --example junit_reporter_example -p data-bridge-test

      - name: Publish Test Results
        uses: dorny/test-reporter@v1
        if: always()
        with:
          name: Test Results
          path: test-results.xml
          reporter: java-junit
```

### Advanced Integration with Multiple Reporters

```yaml
- name: Upload test results
  uses: actions/upload-artifact@v4
  if: always()
  with:
    name: test-results
    path: |
      test-results.xml
      test-report.md
      test-report.html

- name: Comment PR with results
  uses: EnricoMi/publish-unit-test-result-action@v2
  if: always() && github.event_name == 'pull_request'
  with:
    files: test-results.xml
    check_name: Test Results
```

### Test Summary in GitHub Actions

```yaml
- name: Test Summary
  uses: test-summary/action@v2
  if: always()
  with:
    paths: test-results.xml
```

## GitLab CI

### Basic Integration

Create or update `.gitlab-ci.yml`:

```yaml
test:
  stage: test
  image: rust:latest
  script:
    - cargo build --release
    - cargo run --example junit_reporter_example -p data-bridge-test
  artifacts:
    when: always
    reports:
      junit: test-results.xml
    paths:
      - test-results.xml
      - test-report.md
    expire_in: 30 days
```

### With Coverage

```yaml
test:
  stage: test
  image: rust:latest
  before_script:
    - rustup component add llvm-tools-preview
    - cargo install cargo-llvm-cov
  script:
    - cargo llvm-cov --all-features --lcov --output-path lcov.info
    - cargo run --example junit_reporter_example -p data-bridge-test
  coverage: '/\d+\.\d+% coverage/'
  artifacts:
    reports:
      junit: test-results.xml
      coverage_report:
        coverage_format: cobertura
        path: lcov.info
```

### Display Test Results in Merge Requests

GitLab automatically displays JUnit XML results in the merge request UI when you specify:

```yaml
artifacts:
  reports:
    junit: test-results.xml
```

## Jenkins

### Using JUnit Plugin

```groovy
pipeline {
    agent any

    stages {
        stage('Build') {
            steps {
                sh 'cargo build --release'
            }
        }

        stage('Test') {
            steps {
                sh 'cargo run --example junit_reporter_example -p data-bridge-test'
            }
            post {
                always {
                    // Publish JUnit test results
                    junit 'test-results.xml'

                    // Archive additional reports
                    archiveArtifacts artifacts: 'test-report.md,test-report.html', allowEmptyArchive: true
                }
            }
        }
    }
}
```

### With HTML Publisher

```groovy
stage('Test') {
    steps {
        sh 'cargo run --example junit_reporter_example -p data-bridge-test'
    }
    post {
        always {
            junit 'test-results.xml'

            publishHTML([
                allowMissing: false,
                alwaysLinkToLastBuild: true,
                keepAll: true,
                reportDir: '.',
                reportFiles: 'test-report.html',
                reportName: 'Test Report'
            ])
        }
    }
}
```

## CircleCI

### Basic Integration

Create or update `.circleci/config.yml`:

```yaml
version: 2.1

jobs:
  test:
    docker:
      - image: rust:latest
    steps:
      - checkout

      - run:
          name: Build
          command: cargo build --release

      - run:
          name: Run tests
          command: cargo run --example junit_reporter_example -p data-bridge-test

      - store_test_results:
          path: .

      - store_artifacts:
          path: test-results.xml
          destination: test-results

      - store_artifacts:
          path: test-report.html
          destination: test-report

workflows:
  version: 2
  test:
    jobs:
      - test
```

### With Caching

```yaml
- restore_cache:
    keys:
      - cargo-cache-{{ checksum "Cargo.lock" }}
      - cargo-cache-

- run:
    name: Run tests
    command: cargo run --example junit_reporter_example -p data-bridge-test

- save_cache:
    key: cargo-cache-{{ checksum "Cargo.lock" }}
    paths:
      - ~/.cargo
      - target
```

## Example Output

### JUnit XML Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuite name="ExampleTestSuite" tests="7" failures="1" errors="1" skipped="1" time="8.750" timestamp="2024-01-06T10:30:00Z">
  <testcase name="test_database_connection" classname="database_tests::test_database_connection" time="0.150">
  </testcase>
  <testcase name="test_invalid_input" classname="validation_tests::test_invalid_input" time="0.075" file="tests/validation_tests.rs" line="102">
    <failure message="Expected validation error, but operation succeeded">at validation_tests.rs:102
at test_runner::execute_test
at main</failure>
  </testcase>
  <testcase name="test_network_timeout" classname="network_tests::test_network_timeout" time="5.000" file="tests/network_tests.rs" line="67">
    <error type="Error" message="Connection timeout after 5000ms">NetworkError: Connection timeout
at network_tests.rs:67
at tokio::runtime::block_on</error>
  </testcase>
  <testcase name="test_experimental_feature" classname="test_experimental_feature" time="0.000" file="tests/experimental_tests.rs" line="15">
    <skipped message="Feature not yet implemented" />
  </testcase>
</testsuite>
```

### What CI/CD Systems Display

- **GitHub Actions**: Test results appear in the "Checks" tab with pass/fail status
- **GitLab**: Results shown in merge request with expandable test details
- **Jenkins**: Blue Ocean UI shows test trends and failure details
- **CircleCI**: Test insights page with timing and failure analysis

## Troubleshooting

### Issue: XML file not found

**Symptom**: CI fails with "test-results.xml not found"

**Solution**:
```bash
# Verify the example runs successfully
cargo run --example junit_reporter_example -p data-bridge-test

# Check file was created
ls -la test-results.xml

# Ensure correct path in CI configuration
# The file is created in the current working directory
```

### Issue: Invalid XML format

**Symptom**: CI rejects the XML file

**Solution**:
- The reporter automatically escapes XML special characters
- Verify the XML is valid:
  ```bash
  xmllint --noout test-results.xml
  ```
- If using custom test names, avoid XML special characters: `< > & " '`

### Issue: Tests not appearing in CI UI

**Symptom**: XML uploaded but no tests shown

**Solution**:
1. Verify the JUnit XML path matches the CI configuration
2. Check that the test reporter plugin/action is installed
3. Ensure the XML follows JUnit format (not JUnit 5/Jupiter format)
4. Some CI systems require specific artifact/report configuration:

   **GitHub Actions**:
   ```yaml
   - uses: dorny/test-reporter@v1
     with:
       reporter: java-junit  # Important: specify reporter type
   ```

   **GitLab**:
   ```yaml
   artifacts:
     reports:
       junit: test-results.xml  # Must be under 'reports:' key
   ```

### Issue: Time values incorrect

**Symptom**: Test durations show as 0 or wrong values

**Solution**:
- Times are in seconds with 3 decimal places
- The reporter converts milliseconds to seconds automatically
- Verify your test duration measurements are in milliseconds

### Issue: File paths not showing

**Symptom**: CI doesn't link to source files

**Solution**:
```rust
let mut meta = TestMeta::new("test_name");
meta.file_path = Some("tests/my_test.rs".to_string());
meta.line_number = Some(42);
```

Ensure file paths are relative to the project root.

### Issue: Special characters in test names

**Symptom**: XML parsing errors or garbled output

**Solution**:
The reporter automatically escapes:
- `<` → `&lt;`
- `>` → `&gt;`
- `&` → `&amp;`
- `"` → `&quot;`
- `'` → `&apos;`

If you see issues, verify test names don't contain already-escaped sequences.

## Advanced Usage

### Multiple Test Suites

Run the reporter multiple times with different suite names:

```rust
// Suite 1: Unit tests
let unit_report = TestReport::new("UnitTests", unit_results);
let xml1 = Reporter::junit().generate(&unit_report);
std::fs::write("unit-tests.xml", xml1)?;

// Suite 2: Integration tests
let integration_report = TestReport::new("IntegrationTests", integration_results);
let xml2 = Reporter::junit().generate(&integration_report);
std::fs::write("integration-tests.xml", xml2)?;
```

Then in your CI:
```yaml
artifacts:
  reports:
    junit:
      - unit-tests.xml
      - integration-tests.xml
```

### Combining with Coverage

```rust
use data_bridge_test::reporter::{TestReport, CoverageInfo, FileCoverage};

let mut report = TestReport::new("MyTests", results);

// Add coverage data
let coverage = CoverageInfo {
    total_statements: 1000,
    covered_statements: 850,
    coverage_percent: 85.0,
    files: vec![
        FileCoverage {
            path: "src/lib.rs".to_string(),
            statements: 100,
            covered: 85,
            coverage_percent: 85.0,
            missing_lines: vec![42, 43, 50],
        }
    ],
    uncovered_files: vec![],
};

report.set_coverage(coverage);

// Generate both JUnit XML and HTML with coverage
let junit_xml = Reporter::junit().generate(&report);
let html = Reporter::html().generate(&report);  // Includes coverage visualization
```

### Custom Environment Info

```rust
use data_bridge_test::reporter::EnvironmentInfo;

let env = EnvironmentInfo {
    python_version: Some("3.12.0".to_string()),
    rust_version: Some("1.75.0".to_string()),
    platform: Some("Linux x86_64".to_string()),
    hostname: Some("ci-runner-01".to_string()),
};

let report = TestReport::new("MyTests", results)
    .with_environment(env);
```

## Best Practices

1. **Always use `if: always()`** in CI to upload results even if tests fail
2. **Set artifact retention** to balance storage costs with debugging needs
3. **Use descriptive suite names** to identify test types (unit, integration, e2e)
4. **Include file paths and line numbers** for better CI integration
5. **Upload multiple report formats** (XML for CI parsing, HTML for human review)
6. **Archive reports as artifacts** for historical analysis

## Reference

- JUnit XML Format: https://llg.cubic.org/docs/junit/
- GitHub Test Reporter: https://github.com/dorny/test-reporter
- GitLab CI Testing: https://docs.gitlab.com/ee/ci/testing/
- Jenkins JUnit Plugin: https://plugins.jenkins.io/junit/
- CircleCI Test Results: https://circleci.com/docs/collect-test-data/
