# Observability Feature Guide

This document explains how to enable and use the optional OpenTelemetry observability features in data-bridge.

## Overview

The `observability` feature is **optional** and provides:
- OpenTelemetry distributed tracing
- W3C TraceContext propagation (Rust → Python)
- OTLP export to OpenTelemetry Collector
- GCP Cloud Trace integration
- Structured logging

## Why is it Optional?

The observability feature is disabled by default for several reasons:

1. **Reduced Dependencies**: OpenTelemetry adds significant dependencies (protobuf, gRPC, etc.)
2. **Faster Compilation**: Without OpenTelemetry, builds are faster
3. **Smaller Binary Size**: Production builds without observability are more lightweight
4. **Flexibility**: Users who don't need distributed tracing don't pay the cost

## Installation

### Option 1: Install without Observability (Default)

```bash
# Rust crate (default features)
cargo build -p data-bridge-api

# Python package (minimal dependencies)
pip install data-bridge
```

### Option 2: Install with Observability

#### Rust Crate

```bash
# Build with observability feature
cargo build -p data-bridge-api --features observability

# Or add to your Cargo.toml
[dependencies]
data-bridge-api = { version = "0.1", features = ["observability"] }
```

#### Python Package

```bash
# Install Python package with observability dependencies
pip install "data-bridge[observability]"

# Or with uv
uv pip install "data-bridge[observability]"

# Build from source with observability
maturin develop --features observability
pip install opentelemetry-api opentelemetry-sdk opentelemetry-exporter-otlp
```

## Usage

### With Observability Enabled

```rust
use data_bridge_api::{Server, ServerConfig, TelemetryConfig, init_telemetry};

// Initialize telemetry
let telemetry_config = TelemetryConfig {
    service_name: "my-service".to_string(),
    service_version: "1.0.0".to_string(),
    otlp_endpoint: "http://localhost:4317".to_string(),
    json_logging: true,
    sampling_rate: 1.0,
};

init_telemetry(telemetry_config.clone())?;

// Create server with telemetry
let config = ServerConfig::new("127.0.0.1:8000")
    .with_telemetry(telemetry_config);

let server = Server::new(router, config);
server.serve().await?;
```

### Without Observability (Default)

```rust
use data_bridge_api::{Server, ServerConfig};

// TelemetryConfig and init_telemetry are NOT available
// Server works normally without distributed tracing

let config = ServerConfig::new("127.0.0.1:8000");
let server = Server::new(router, config);
server.serve().await?;
```

## Python Integration

### With Observability

```python
from data_bridge.pyloop import App, OpenTelemetryMiddleware

app = App()

# Add OpenTelemetry middleware
# Requires: pip install "data-bridge[observability]"
app.add_middleware(OpenTelemetryMiddleware(tracer_name="my-service"))

@app.get("/")
async def root(request):
    # Spans are automatically created and linked
    return {"message": "Hello with tracing!"}

app.serve()
```

### Without Observability

```python
from data_bridge.pyloop import App

app = App()

# OpenTelemetryMiddleware is available but will log a warning
# if OpenTelemetry packages are not installed

@app.get("/")
async def root(request):
    return {"message": "Hello without tracing!"}

app.serve()
```

## Feature Detection

Check if observability is available at runtime:

```rust
#[cfg(feature = "observability")]
{
    // Observability code
    use data_bridge_api::telemetry::{TelemetryConfig, init_telemetry};
    // ...
}

#[cfg(not(feature = "observability"))]
{
    println!("Observability feature not enabled");
}
```

## Environment Variables

When observability is enabled, configure via environment variables:

```bash
# OpenTelemetry configuration
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4317"
export OTEL_SERVICE_NAME="data-bridge-api"
export OTEL_SERVICE_VERSION="0.1.0"
export OTEL_RESOURCE_ATTRIBUTES="deployment.environment=production"

# Logging format
export LOG_FORMAT="json"  # or "plain" for development
```

## Deployment Scenarios

### Scenario 1: Development (No Observability)

```bash
# Fast builds, minimal dependencies
cargo build -p data-bridge-api
pip install data-bridge

# No trace export, basic logging only
```

### Scenario 2: Local Testing (With Observability)

```bash
# Build with observability
cargo build -p data-bridge-api --features observability
pip install "data-bridge[observability]"

# Run local OTel Collector + Jaeger
docker-compose -f deploy/docker-compose.otel.yml up -d

# Set environment variables
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4317"

# Run tests
python tests/verify_tracing.py
```

### Scenario 3: Production on GCP (With Observability)

```bash
# Build production image with observability
maturin build --release --features observability
pip install data-bridge-*.whl
pip install "data-bridge[observability]"

# Deploy to GKE with OTel Collector sidecar
kubectl apply -f deploy/gcp/k8s-manifests.yaml

# Traces automatically exported to Cloud Trace
```

## Performance Impact

### With Observability Disabled (Default)
- **Build time**: ~20-30% faster
- **Binary size**: ~2-3 MB smaller
- **Runtime overhead**: None (zero cost)
- **Memory**: Lower footprint

### With Observability Enabled
- **Build time**: Additional ~30 seconds (first build)
- **Binary size**: ~2-3 MB larger (OTLP/protobuf)
- **Runtime overhead**: <1% (trace creation and export)
- **Memory**: ~50-100 MB for OTLP exporter

## Migration Guide

### Upgrading from Previous Versions

If you previously had observability as a required dependency:

1. **Update Cargo.toml** (if using Rust API):
   ```toml
   [dependencies]
   data-bridge-api = { version = "0.1", features = ["observability"] }
   ```

2. **Update Python dependencies**:
   ```bash
   pip install "data-bridge[observability]"
   ```

3. **Update build scripts**:
   ```bash
   # Before
   maturin develop

   # After (with observability)
   maturin develop --features observability
   pip install opentelemetry-api opentelemetry-sdk opentelemetry-exporter-otlp
   ```

## Troubleshooting

### Issue: "TelemetryConfig not found"

**Cause**: Trying to use observability types without the feature enabled.

**Solution**:
```bash
cargo build --features observability
```

### Issue: "cannot find module telemetry in crate"

**Cause**: Code tries to import `telemetry` module when feature is disabled.

**Solution**: Wrap imports with feature flag:
```rust
#[cfg(feature = "observability")]
use data_bridge_api::telemetry::TelemetryConfig;
```

### Issue: Python "OpenTelemetry not installed" warning

**Cause**: Using `OpenTelemetryMiddleware` without installing dependencies.

**Solution**:
```bash
pip install "data-bridge[observability]"
```

## FAQ

### Q: Should I enable observability in production?

**A**: It depends on your needs:
- **Enable** if you need distributed tracing, APM, or troubleshooting capabilities
- **Disable** if you're optimizing for minimal footprint or don't use tracing

### Q: Can I enable observability at runtime?

**A**: No, it must be enabled at compile time. However, Python middleware will gracefully handle missing dependencies.

### Q: Does disabling observability affect basic logging?

**A**: No. Basic `tracing` logs are always available. Only OpenTelemetry-specific features (span export, W3C propagation) are disabled.

### Q: What's the recommended setup for GCP?

**A**: Enable observability feature and use the sidecar pattern with OTel Collector:
```bash
maturin build --release --features observability
kubectl apply -f deploy/gcp/k8s-manifests.yaml
```

## References

- [OpenTelemetry Rust Documentation](https://docs.rs/opentelemetry/)
- [OpenTelemetry Python Documentation](https://opentelemetry.io/docs/instrumentation/python/)
- [GCP Cloud Trace](https://cloud.google.com/trace/docs)
- [Testing Guide](./TESTING.md)
