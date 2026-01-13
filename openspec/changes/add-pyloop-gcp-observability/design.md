# Design: PyLoop GCP Observability

## 1. Architecture: The Gateway Pattern

We use the "Rust as Gateway" pattern. Since Rust terminates the HTTP connection, it owns the **Root Span**.

```mermaid
sequenceDiagram
    participant Client
    participant RustServer as Rust HTTP Server
    participant PyLoop as PyLoop (Bridge)
    participant Python as Python Middleware
    participant OTel as OTel Collector

    Client->>RustServer: HTTP Request
    activate RustServer
    RustServer->>RustServer: Start Root Span (trace_id: A)
    RustServer->>RustServer: Inject Context (TraceParent) into Headers
    RustServer->>PyLoop: Dispatch Request (with headers)
    activate PyLoop
    PyLoop->>Python: Call Handler
    activate Python
    Python->>Python: Middleware Extracts Context
    Python->>Python: Start Child Span (trace_id: A, parent: Root)
    Python->>Python: Process Logic
    Python-->>PyLoop: Return Response
    deactivate Python
    PyLoop-->>RustServer: Return Response
    deactivate PyLoop
    RustServer->>RustServer: End Root Span
    RustServer-->>Client: HTTP Response
    deactivate RustServer

    RustServer--)OTel: Export Span (Root)
    Python--)OTel: Export Span (Child)
```

## 2. Rust Implementation Details

### Dependencies
```toml
[dependencies]
opentelemetry = "0.21"
opentelemetry_sdk = { version = "0.21", features = ["rt-tokio"] }
opentelemetry-otlp = "0.14"
tracing = "0.1"
tracing-opentelemetry = "0.22"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

### Context Injection
We must manually inject the context because `data-bridge-api` converts `HyperRequest` to `SerializableRequest` manually.
We will use `opentelemetry::global::get_text_map_propagator` to inject into a `HeaderMap`, then copy those headers to `SerializableRequest`.

## 3. Python Implementation Details

### Dependencies
```toml
[project.dependencies]
opentelemetry-api = "^1.20.0"
opentelemetry-sdk = "^1.20.0"
opentelemetry-exporter-otlp = "^1.20.0"
```

### Middleware Logic
The `OpenTelemetryMiddleware` must use `TraceContextTextMapPropagator` from `opentelemetry.trace.propagation` to extract the context from the `request["headers"]` dict.

## 4. Infrastructure (GKE)

### OTel Collector Configuration (Sidecar)
```yaml
receivers:
  otlp:
    protocols:
      grpc:
      http:

exporters:
  googlecloud:
    # Google Cloud Operations
    project: "my-project-id"

processors:
  batch:
  resourcedetection:
    detectors: [gcp]

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [resourcedetection, batch]
      exporters: [googlecloud]
```
