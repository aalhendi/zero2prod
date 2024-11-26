use tokio::task::JoinHandle;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{
    fmt::MakeWriter, layer::SubscriberExt, registry::LookupSpan, EnvFilter, Registry,
};
#[cfg(feature = "open-telemetry")]
use {
    crate::configuration::OpenTelemetrySettings,
    opentelemetry::{trace::TracerProvider as _, KeyValue},
    opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge,
    opentelemetry_otlp::{WithExportConfig, WithHttpConfig},
    opentelemetry_sdk::{
        logs::LoggerProvider, metrics::SdkMeterProvider, runtime, trace::TracerProvider, Resource,
    },
    opentelemetry_semantic_conventions::{
        resource::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_NAME, SERVICE_VERSION},
        SCHEMA_URL,
    },
    tracing_opentelemetry::OpenTelemetryLayer,
};

#[cfg(feature = "open-telemetry")]
pub struct OtelGuard {
    meter_provider: SdkMeterProvider,
    logger_provider: LoggerProvider,
    tracer_provider: TracerProvider,
}

#[cfg(feature = "open-telemetry")]
impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("Error shutting down meter provider: {err:?}");
        }

        if let Err(err) = self.logger_provider.shutdown() {
            eprintln!("Error shutting down logger provider: {err:?}");
        }

        if let Err(err) = self.tracer_provider.shutdown() {
            eprintln!("Error shutting down tracer provider: {err:?}");
        }
    }
}

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// `impl Subscriber` as return type to avoid having to spell out the actual type of returned subscriber.
/// Also specify it implements `Send` and `Sync` to make it possible to pass to `init_subscriber`.
/// Finally, specify it implements LookupSpan to optionally add more layers to the subscriber
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync + for<'span> LookupSpan<'span>
where
    // Basically specifying that type Sink implements MakeWriter trait for all choices of lifetime param `'a`
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // Either set `RUST_LOG` env var or default to `info` level. Init calls `set_logger()`
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

#[cfg(feature = "open-telemetry")]
pub fn add_otel_to_subscriber<S>(
    subscriber: S,
    settings: &OpenTelemetrySettings,
) -> (impl Subscriber + Send + Sync, OtelGuard)
where
    S: Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
{
    let meter_provider = SdkMeterProvider::default();
    let tracer_provider = init_tracer(settings);
    let otel_trace_layer =
        OpenTelemetryLayer::new(tracer_provider.tracer("tracing-otel-subscriber"));
    let logger_provider = init_logger(settings);
    let otel_log_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    let guard = OtelGuard {
        meter_provider,
        logger_provider,
        tracer_provider,
    };

    let subscriber = subscriber.with(otel_trace_layer).with(otel_log_layer);

    (subscriber, guard)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // Redirect all `log`'s events to our subscriber
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber"); // specify default subscriber to process tacing spans
}

/// Inherits Singature from tokio::task::spawn_blocking
pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    // Explicitly attach current span to newly spawned thread to maintain span context.
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}

#[cfg(feature = "open-telemetry")]
/// Creates a Resource that captures information about the entity for which telemetry is recorded.
fn resource() -> Resource {
    let environment = std::env::var("APP_ENVIRONMENT").unwrap_or_else(|_| String::from("local"));
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, environment),
        ],
        SCHEMA_URL,
    )
}

#[cfg(feature = "open-telemetry")]
fn init_tracer(settings: &OpenTelemetrySettings) -> TracerProvider {
    use opentelemetry_sdk::trace::Config;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(settings.trace_full_url())
        .with_headers(settings.headers())
        .with_timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

    TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_config(Config::default().with_resource(resource()))
        .build()
}

#[cfg(feature = "open-telemetry")]
fn init_logger(settings: &OpenTelemetrySettings) -> LoggerProvider {
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_endpoint(settings.log_full_url())
        .with_headers(settings.headers())
        .with_timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

    LoggerProvider::builder()
        .with_resource(resource())
        .with_batch_exporter(exporter, runtime::Tokio)
        .build()
}
