# OpenTelemetry Tracing - Local Testing Guide

This guide explains how to verify OpenTelemetry distributed tracing locally before deploying to GCP.

## Feature Flag Requirement

The observability features are **optional** and must be explicitly enabled:

```bash
# Build Rust with observability
cargo build -p data-bridge-api --features observability

# Or use maturin for Python package
maturin develop --features observability

# Install Python dependencies
pip install "data-bridge[observability]"
# or: pip install opentelemetry-api opentelemetry-sdk opentelemetry-exporter-otlp
```

For more details on the observability feature, see [OBSERVABILITY.md](./OBSERVABILITY.md).

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   PyLoop HTTP Server                        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Rust Layer (Gateway)                                │  │
│  │  - Creates root span: "http.request"                 │  │
│  │  - Injects trace context into headers               │  │
│  │  - Span attributes: http.method, http.target, etc.  │  │
│  └────────────────┬─────────────────────────────────────┘  │
│                   │ W3C TraceContext Propagation            │
│                   ▼                                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Python Layer (Handler)                              │  │
│  │  - Extracts trace context from headers              │  │
│  │  - Creates child span: "pyloop.request"             │  │
│  │  - Inherits trace_id from Rust root span            │  │
│  └────────────────┬─────────────────────────────────────┘  │
└───────────────────┼──────────────────────────────────────────┘
                    │ OTLP (gRPC or HTTP)
                    ▼
      ┌──────────────────────────────┐
      │  OpenTelemetry Collector     │
      │  - Receives traces via OTLP  │
      │  - Batch processing          │
      │  - Resource detection        │
      └──────────────┬───────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
  ┌──────────┐            ┌──────────┐
  │  Jaeger  │            │ Console  │
  │  (UI)    │            │ (Debug)  │
  └──────────┘            └──────────┘
```

## Prerequisites

1. **Install Python dependencies:**
   ```bash
   pip install opentelemetry-api opentelemetry-sdk opentelemetry-exporter-otlp httpx
   ```

2. **Build data-bridge with PyLoop support:**
   ```bash
   maturin develop --features pyloop,api
   ```

3. **Install Docker and Docker Compose** (for local OTel Collector)

## Test Methods

### Method 1: Console Exporter (Quick Verification)

This method uses `ConsoleSpanExporter` to print spans directly to stdout. No external dependencies required.

**Run:**
```bash
python tests/verify_tracing.py
```

**Expected Output:**
```
Starting PyLoop server in background...
Waiting for server to be ready...
Sending test requests...

[Test 1] GET /
  Status: 200
  Response: {'message': 'PyLoop with distributed tracing', ...}
  ✓ Test 1 passed

{
  "name": "http.request",
  "context": {
    "trace_id": "0x1234567890abcdef1234567890abcdef",
    "span_id": "0x1234567890abcdef",
    "trace_state": "[]"
  },
  "kind": "SpanKind.SERVER",
  "parent_id": null,
  "start_time": "2024-01-01T12:00:00.000000Z",
  "end_time": "2024-01-01T12:00:00.100000Z",
  "status": {
    "status_code": "UNSET"
  },
  "attributes": {
    "http.method": "GET",
    "http.target": "/",
    "http.scheme": "http",
    "otel.kind": "server"
  },
  ...
}

{
  "name": "pyloop.request",
  "context": {
    "trace_id": "0x1234567890abcdef1234567890abcdef",  # ← Same trace_id!
    "span_id": "0xabcdef1234567890",
    "trace_state": "[]"
  },
  "kind": "SpanKind.INTERNAL",
  "parent_id": "0x1234567890abcdef",  # ← Points to Rust span!
  "start_time": "2024-01-01T12:00:00.050000Z",
  "end_time": "2024-01-01T12:00:00.090000Z",
  ...
}
```

**What to verify:**
- ✓ Both spans share the same `trace_id`
- ✓ Python span's `parent_id` matches Rust span's `span_id`
- ✓ Span names: `http.request` (Rust) and `pyloop.request` (Python)
- ✓ Span attributes include HTTP method, route, status code

### Method 2: Local OTel Collector + Jaeger (Full Stack)

This method runs a complete observability stack locally using Docker Compose.

**1. Start OTel Collector and Jaeger:**
```bash
cd deploy
docker-compose -f docker-compose.otel.yml up -d
```

**2. Verify containers are running:**
```bash
docker ps
```

Expected output:
```
CONTAINER ID   IMAGE                                      PORTS
abc123def456   otel/opentelemetry-collector-contrib:...   4317-4318,8888,13133,55679
def789ghi012   jaegertracing/all-in-one:1.51              14250,14268,16686
```

**3. Update verify_tracing.py to use OTLP exporter:**

Edit `tests/verify_tracing.py`, replace `ConsoleSpanExporter` with `OTLPSpanExporter`:

```python
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter

# Replace this:
console_exporter = ConsoleSpanExporter()
span_processor = BatchSpanProcessor(console_exporter)

# With this:
otlp_exporter = OTLPSpanExporter(
    endpoint="http://localhost:4317",
    insecure=True
)
span_processor = BatchSpanProcessor(otlp_exporter)
```

**4. Run the verification script:**
```bash
python tests/verify_tracing.py
```

**5. Open Jaeger UI:**
```
http://localhost:16686
```

**6. View traces in Jaeger:**
- Select service: `data-bridge-api`
- Click "Find Traces"
- Click on a trace to see the full span hierarchy

**Expected Jaeger visualization:**
```
http.request (Rust) ───┐  Duration: 100ms
                       │
                       └─► pyloop.request (Python)  Duration: 40ms
```

**7. Check OTel Collector logs:**
```bash
docker logs otel-collector
```

Expected output:
```
2024-01-01T12:00:00.000Z  info  TracesExporter  {"kind": "exporter", "data_type": "traces", "name": "otlp/jaeger", "traces_sent": 10, "spans_sent": 20}
```

**8. Stop the stack:**
```bash
docker-compose -f docker-compose.otel.yml down
```

## Verification Checklist

Use this checklist to verify distributed tracing works correctly:

### Trace Context Propagation
- [ ] Rust creates root span with trace_id
- [ ] Rust injects trace context into HTTP headers (traceparent, tracestate)
- [ ] Python extracts trace context from headers
- [ ] Python creates child span with same trace_id
- [ ] Python span's parent_id matches Rust span's span_id

### Span Attributes
- [ ] Rust span includes: http.method, http.target, http.scheme, otel.kind
- [ ] Python span includes: http.method, http.route, http.status_code
- [ ] Both spans include resource attributes: service.name, service.version

### Export and Visualization
- [ ] Spans are exported to OTel Collector via OTLP
- [ ] OTel Collector forwards spans to Jaeger
- [ ] Traces visible in Jaeger UI with correct parent-child relationship
- [ ] Span timing makes sense (parent >= child duration)

### Error Handling
- [ ] Failed requests (4xx, 5xx) create spans with error status
- [ ] Exception details captured in span events
- [ ] Error spans properly linked to parent spans

## Troubleshooting

### Issue: "OpenTelemetry not installed"
**Solution:**
```bash
pip install opentelemetry-api opentelemetry-sdk opentelemetry-exporter-otlp
```

### Issue: "Connection refused to localhost:4317"
**Cause:** OTel Collector not running

**Solution:**
```bash
docker-compose -f deploy/docker-compose.otel.yml up -d
docker ps  # Verify containers are running
```

### Issue: "No traces visible in Jaeger"
**Possible causes:**
1. OTel Collector not receiving traces
2. Jaeger not receiving traces from collector
3. Service name mismatch

**Debug steps:**
```bash
# Check OTel Collector logs
docker logs otel-collector

# Check Jaeger logs
docker logs jaeger

# Verify OTel Collector health
curl http://localhost:13133
```

### Issue: "Trace IDs don't match between Rust and Python"
**Cause:** Trace context not properly propagated

**Debug:**
1. Check that `inject_trace_context()` is called in Rust server.rs
2. Verify Python middleware uses `TraceContextTextMapPropagator`
3. Check HTTP headers contain `traceparent` header

**Solution:**
```bash
# Enable debug logging to see headers
RUST_LOG=debug python tests/verify_tracing.py
```

### Issue: "Python span has no parent_id"
**Cause:** Context extraction failed in Python

**Debug:**
```python
# Add debug logging in OpenTelemetryMiddleware
carrier = {k.lower(): v for k, v in headers.items()}
print(f"Extracted carrier: {carrier}")

ctx = self.propagator.extract(carrier=carrier)
print(f"Extracted context: {ctx}")
```

## Next Steps

After local verification succeeds:

1. **Deploy to GKE:**
   ```bash
   kubectl apply -f deploy/gcp/k8s-manifests.yaml
   ```

2. **View traces in Google Cloud Trace:**
   ```
   https://console.cloud.google.com/traces
   ```

3. **Monitor with Cloud Monitoring:**
   ```
   https://console.cloud.google.com/monitoring
   ```

## References

- [OpenTelemetry Python Docs](https://opentelemetry.io/docs/instrumentation/python/)
- [W3C Trace Context Specification](https://www.w3.org/TR/trace-context/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [OTel Collector Configuration](https://opentelemetry.io/docs/collector/configuration/)
