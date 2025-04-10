use http::HeaderMap;
use opentelemetry::{
	Context, InstrumentationScope, KeyValue,
	baggage::BaggageExt,
	global::{self, BoxedTracer},
	logs::LogRecord,
	propagation::TextMapCompositePropagator,
	trace::{FutureExt, Span, SpanKind, TraceContextExt, Tracer},
};
use opentelemetry_http::{Bytes, HeaderExtractor};
use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{
	error::OTelSdkResult,
	logs::{LogProcessor, SdkLogRecord, SdkLoggerProvider},
	propagation::{BaggagePropagator, TraceContextPropagator},
	trace::{SdkTracerProvider, SpanProcessor},
};
use std::{convert::Infallible, net::SocketAddr, sync::OnceLock};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn get_tracer() -> &'static BoxedTracer {
	static TRACER: OnceLock<BoxedTracer> = OnceLock::new();
	TRACER.get_or_init(|| global::tracer("mcp-proxy"))
}

// Utility function to extract the context from the incoming request headers
pub fn extract_context_from_request(req: &HeaderMap) -> Context {
	global::get_text_map_propagator(|propagator| propagator.extract(&HeaderExtractor(req)))
}

fn get_resource() -> Resource {
	static RESOURCE: OnceLock<Resource> = OnceLock::new();
	RESOURCE
		.get_or_init(|| {
			Resource::builder()
				.with_service_name("basic-otlp-example-grpc")
				.build()
		})
		.clone()
}

/// A custom span processor that enriches spans with baggage attributes. Baggage
/// information is not added automatically without this processor.
#[derive(Debug)]
struct EnrichWithBaggageSpanProcessor;
impl SpanProcessor for EnrichWithBaggageSpanProcessor {
	fn force_flush(&self) -> OTelSdkResult {
		Ok(())
	}

	fn shutdown(&self) -> OTelSdkResult {
		Ok(())
	}

	fn on_start(&self, span: &mut opentelemetry_sdk::trace::Span, cx: &Context) {
		for (kk, vv) in cx.baggage().iter() {
			span.set_attribute(KeyValue::new(kk.clone(), vv.0.clone()));
		}
	}

	fn on_end(&self, _span: opentelemetry_sdk::trace::SpanData) {}
}

pub fn init_tracer() -> SdkTracerProvider {
	let baggage_propagator = BaggagePropagator::new();
	let trace_context_propagator = TraceContextPropagator::new();
	let composite_propagator = TextMapCompositePropagator::new(vec![
		Box::new(baggage_propagator),
		Box::new(trace_context_propagator),
	]);

	global::set_text_map_propagator(composite_propagator);

	let exporter = SpanExporter::builder()
		.with_tonic()
		// .with_endpoint("http://localhost:4318/v1/traces")
		.build()
		.expect("Failed to create span exporter");
	let provider = SdkTracerProvider::builder()
		.with_span_processor(EnrichWithBaggageSpanProcessor)
		.with_resource(get_resource())
		.with_batch_exporter(exporter)
		.build();

	global::set_tracer_provider(provider.clone());
	provider
}
