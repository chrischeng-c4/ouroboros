# Implementation Tasks

## 1. Rust Core (Observability Gateway)
- [x] 1.1 Add dependencies to `crates/data-bridge-api/Cargo.toml`: `tracing-opentelemetry`, `opentelemetry`, `opentelemetry-otlp`, `opentelemetry_sdk`.
- [x] 1.2 Implement `init_telemetry()` function in `crates/data-bridge-api/src/telemetry.rs` (new file) to configure the global tracer provider with OTLP exporter.
- [x] 1.3 Add `TelemetryConfig` to `ServerConfig` in `crates/data-bridge-api/src/server.rs`.
- [x] 1.4 Instrument `server.rs`: Wrap `handle_request` in a root span using `tracing`.
- [x] 1.5 Implement context injection: Before converting to `SerializableRequest`, inject the current trace context into the HTTP headers so Python can inherit it.

## 2. Python Core (Handler Instrumentation)
- [x] 2.1 Add `opentelemetry-api` and `opentelemetry-sdk` to `pyproject.toml`.
- [x] 2.2 Create `OpenTelemetryMiddleware` in `python/data_bridge/pyloop/middleware.py` (refactor from `__init__.py`).
- [x] 2.3 Implement `process_request` in `OpenTelemetryMiddleware`:
    - Extract trace context from `request.headers` using `TraceContextTextMapPropagator`.
    - Start a new span `pyloop.request` as a child of the extracted context.
    - Set span attributes (http.method, http.route).
- [x] 2.4 Update `python/data_bridge/pyloop/__init__.py` to export the new middleware.

## 3. Infrastructure & Config
- [x] 3.1 Create `deploy/gcp/otel-collector-config.yaml` with GCP exporter configuration.
- [x] 3.2 Create `deploy/gcp/k8s-manifests.yaml` showing Sidecar deployment pattern.

## 4. Verification
- [x] 4.1 Create a test script `verify_tracing.py` that starts the server and sends a request, checking logs for trace IDs.
- [x] 4.2 Run local OTel collector (using Docker) and verify spans are received.
