//! Telemetry and distributed tracing for KaosNet.
//!
//! Provides OpenTelemetry integration with support for:
//! - OTLP export to Jaeger, Tempo, or other collectors
//! - Console output for development
//! - Structured logging with spans
//!
//! # Example
//!
//! ```rust,ignore
//! use kaosnet::telemetry::{init_tracing, TracingConfig};
//!
//! // Initialize with defaults (console output)
//! init_tracing(TracingConfig::default());
//!
//! // Or with OTLP export
//! init_tracing(TracingConfig {
//!     service_name: "kaosnet".into(),
//!     otlp_endpoint: Some("http://localhost:4317".into()),
//!     log_level: "info".into(),
//!     json_output: false,
//! });
//! ```

use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{Config, Sampler},
    Resource,
};
use opentelemetry::KeyValue;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Tracing configuration.
#[derive(Clone, Debug)]
pub struct TracingConfig {
    /// Service name for traces.
    pub service_name: String,
    /// OTLP endpoint (e.g., "http://localhost:4317"). None for console-only.
    pub otlp_endpoint: Option<String>,
    /// Log level filter (e.g., "info", "debug", "kaosnet=debug,tower=warn").
    pub log_level: String,
    /// Output logs as JSON (useful for log aggregation).
    pub json_output: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "kaosnet".into(),
            otlp_endpoint: None,
            log_level: "info".into(),
            json_output: false,
        }
    }
}

impl TracingConfig {
    /// Create a new config with service name.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    /// Set OTLP endpoint for trace export.
    pub fn with_otlp(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }

    /// Set log level filter.
    pub fn with_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }

    /// Enable JSON output.
    pub fn with_json(mut self) -> Self {
        self.json_output = true;
        self
    }
}

/// Initialize the tracing subscriber with OpenTelemetry.
///
/// Call this once at application startup.
pub fn init_tracing(config: TracingConfig) -> Option<TracingGuard> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Build layers
    let registry = tracing_subscriber::registry().with(env_filter);

    // Console/stdout layer
    if config.json_output {
        let fmt_layer = fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        if let Some(endpoint) = &config.otlp_endpoint {
            // JSON + OTLP
            let (tracer_provider, guard) = init_otlp_tracer(&config.service_name, endpoint);
            let otel_layer = tracing_opentelemetry::layer()
                .with_tracer(tracer_provider.tracer("kaosnet"));

            registry
                .with(fmt_layer)
                .with(otel_layer)
                .init();

            Some(guard)
        } else {
            // JSON only
            registry.with(fmt_layer).init();
            None
        }
    } else {
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .compact();

        if let Some(endpoint) = &config.otlp_endpoint {
            // Pretty + OTLP
            let (tracer_provider, guard) = init_otlp_tracer(&config.service_name, endpoint);
            let otel_layer = tracing_opentelemetry::layer()
                .with_tracer(tracer_provider.tracer("kaosnet"));

            registry
                .with(fmt_layer)
                .with(otel_layer)
                .init();

            Some(guard)
        } else {
            // Pretty only
            registry.with(fmt_layer).init();
            None
        }
    }
}

/// Initialize OTLP tracer provider.
fn init_otlp_tracer(service_name: &str, endpoint: &str) -> (opentelemetry_sdk::trace::TracerProvider, TracingGuard) {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .build_span_exporter()
        .expect("Failed to create OTLP exporter");

    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION").to_string()),
    ]);

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_config(
            Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(resource),
        )
        .build();

    let guard = TracingGuard {
        provider: provider.clone(),
    };

    (provider, guard)
}

/// Guard that shuts down the tracer provider on drop.
pub struct TracingGuard {
    provider: opentelemetry_sdk::trace::TracerProvider,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Err(e) = self.provider.shutdown() {
            eprintln!("Error shutting down tracer provider: {:?}", e);
        }
    }
}

/// Convenience macro for creating spans with context.
#[macro_export]
macro_rules! span_context {
    ($name:expr, $($field:tt)*) => {
        tracing::info_span!($name, $($field)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = TracingConfig::new("test-service")
            .with_level("debug")
            .with_json();

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.log_level, "debug");
        assert!(config.json_output);
        assert!(config.otlp_endpoint.is_none());
    }

    #[test]
    fn test_config_with_otlp() {
        let config = TracingConfig::new("game-server")
            .with_otlp("http://localhost:4317")
            .with_level("info,kaosnet=debug");

        assert_eq!(config.otlp_endpoint, Some("http://localhost:4317".into()));
    }
}
