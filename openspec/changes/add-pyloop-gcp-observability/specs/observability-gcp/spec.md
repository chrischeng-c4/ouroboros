# Specification: GCP Observability

## ADDED Requirements

### Requirement: OTLP Trace Export
The system SHALL export distributed traces via the OTLP (OpenTelemetry Protocol) gRPC exporter.

#### Scenario: GKE Deployment
- **WHEN** the application is running in a GKE pod with an OTel Sidecar
- **THEN** traces are sent to `localhost:4317` (default)
- **AND** the Sidecar forwards them to Google Cloud Trace

### Requirement: Cross-Language Context Propagation
The system SHALL propagate trace context from the Rust HTTP Server layer to the Python Handler layer.

#### Scenario: Request Handling
- **WHEN** a request is received by the Rust server
- **THEN** a root span is created in Rust
- **AND** the trace context (trace_id, span_id) is injected into the request headers
- **WHEN** the request reaches the Python handler
- **THEN** the Python middleware extracts the context
- **AND** creates a child span linked to the Rust root span

### Requirement: Structured JSON Logging
The system SHALL emit logs in a structured JSON format compatible with Google Cloud Logging when configured.

#### Scenario: Log Entry
- **WHEN** a log message is emitted (e.g., "Request processed")
- **THEN** the output format is JSON
- **AND** it includes `severity`, `message`, `timestamp`
- **AND** it includes `logging.googleapis.com/trace` matching the current trace ID

### Requirement: GCP Resource Detection
The system SHALL detect and attach GCP resource attributes to traces and metrics.

#### Scenario: Startup on GKE
- **WHEN** the application starts
- **THEN** it detects it is running on GKE
- **AND** attaches attributes like `k8s.pod.name`, `k8s.namespace.name`, `cloud.region` to the Resource.
