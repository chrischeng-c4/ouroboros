# Change: Add PyLoop GCP Observability

## Why
The current PyLoop HTTP Server lacks production-grade observability for Google Cloud Platform (GKE).
- **No distributed tracing**: Impossible to debug latency issues across Rust (Server) and Python (Handler) boundaries.
- **Basic logging**: Current `LoggingMiddleware` is unstructured text, making it hard to query in Cloud Logging.
- **Missing metrics**: No Prometheus/Stackdriver metrics for HPA (Horizontal Pod Autoscaling) or alerts.

To run `data-bridge` in production on GKE, we need a robust, low-overhead observability stack that leverages the OpenTelemetry ecosystem and integrates natively with GCP Cloud Operations.

## What Changes
- **Architecture**: Adopt the **OpenTelemetry Gateway** pattern.
    - **Rust**: Acts as the observability gateway, initializing traces, handling sampling, and exporting to the OTel Collector.
    - **Python**: Acts as a "worker" in the trace, attaching to the Rust-initiated span context.
- **Rust Implementation**:
    - Add `tracing-opentelemetry` and `opentelemetry-otlp` to `data-bridge-api`.
    - Implement a `TraceLayer` in the Hyper server to start spans and inject W3C trace context into request headers.
- **Python Implementation**:
    - Add `opentelemetry-api` to Python dependencies.
    - Replace `LoggingMiddleware` with `OpenTelemetryMiddleware` in `data_bridge.pyloop`.
    - Implement propagation to extract trace context from Rust.
- **Infrastructure**:
    - Define OTel Collector configuration (Sidecar/DaemonSet) for GCP.

## Impact
- **Affected Specs**: `observability`, `api-server`.
- **Affected Code**:
    - `crates/data-bridge-api/src/server.rs`: Main integration point.
    - `python/data_bridge/pyloop/__init__.py`: Middleware changes.
    - `Cargo.toml`: New dependencies.
    - `pyproject.toml`: New dependencies.
