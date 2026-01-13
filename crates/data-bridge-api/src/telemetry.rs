use opentelemetry::global;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{self, RandomIdGenerator, Sampler};
use opentelemetry_sdk::{runtime, Resource};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Configuration for OpenTelemetry telemetry
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Service name for the application
    pub service_name: String,

    /// Service version
    pub service_version: String,

    /// OTLP endpoint (default: http://localhost:4317)
    pub otlp_endpoint: String,

    /// Enable structured JSON logging
    pub json_logging: bool,

    /// Trace sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "data-bridge-pyloop".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            otlp_endpoint: "http://localhost:4317".to_string(),
            json_logging: false,
            sampling_rate: 1.0,
        }
    }
}

/// Initialize OpenTelemetry tracing with OTLP exporter
///
/// This sets up:
/// - Global tracer provider with OTLP/gRPC exporter
/// - W3C TraceContext propagation
/// - Resource attributes (service.name, service.version)
/// - Tracing subscriber with OpenTelemetry layer
///
/// # Example
/// ```no_run
/// use data_bridge_api::telemetry::{TelemetryConfig, init_telemetry};
///
/// let config = TelemetryConfig::default();
/// init_telemetry(config).expect("Failed to initialize telemetry");
/// ```
pub fn init_telemetry(config: TelemetryConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Set up W3C TraceContext propagation
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Configure resource attributes for GCP
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
    ]);

    // Configure OTLP exporter
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&config.otlp_endpoint);

    // Configure sampler based on sampling rate
    let sampler = if config.sampling_rate >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sampling_rate <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_rate)
    };

    // Build and install tracer (this also sets the global provider)
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(
            trace::config()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .install_batch(runtime::Tokio)?;

    // Create OpenTelemetry tracing layer
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Configure environment filter (default: info)
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    // Build tracing subscriber
    if config.json_logging {
        // JSON structured logging for production (GCP Cloud Logging)
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(telemetry_layer)
            .with(fmt_layer)
            .init();
    } else {
        // Human-readable logging for development
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(telemetry_layer)
            .with(fmt_layer)
            .init();
    }

    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        otlp_endpoint = %config.otlp_endpoint,
        sampling_rate = %config.sampling_rate,
        "Telemetry initialized"
    );

    Ok(())
}

/// Shutdown OpenTelemetry tracer provider
///
/// This ensures all pending spans are exported before shutdown.
/// Call this in your application's graceful shutdown handler.
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, "data-bridge-pyloop");
        assert_eq!(config.otlp_endpoint, "http://localhost:4317");
        assert_eq!(config.sampling_rate, 1.0);
        assert!(!config.json_logging);
    }

    #[test]
    fn test_telemetry_config_custom() {
        let config = TelemetryConfig {
            service_name: "custom-service".to_string(),
            service_version: "1.0.0".to_string(),
            otlp_endpoint: "http://otel-collector:4317".to_string(),
            json_logging: true,
            sampling_rate: 0.5,
        };
        assert_eq!(config.service_name, "custom-service");
        assert_eq!(config.service_version, "1.0.0");
        assert_eq!(config.otlp_endpoint, "http://otel-collector:4317");
        assert!(config.json_logging);
        assert_eq!(config.sampling_rate, 0.5);
    }
}
