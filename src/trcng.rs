use http::HeaderMap;
use opentelemetry::{
	Context, KeyValue,
	baggage::BaggageExt,
	global::{self, BoxedTracer},
	propagation::TextMapCompositePropagator,
	trace::Span,
};
use opentelemetry_http::{HeaderExtractor, HeaderInjector};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::{ExporterBuildError, SpanExporter};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{
	error::OTelSdkResult,
	propagation::{BaggagePropagator, TraceContextPropagator},
	trace::{SdkTracerProvider, SpanProcessor},
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tracing::info;

pub fn get_tracer() -> &'static BoxedTracer {
	static TRACER: OnceLock<BoxedTracer> = OnceLock::new();
	TRACER.get_or_init(|| global::tracer("mcp-proxy"))
}

// Utility function to extract the context from the incoming request headers
pub fn extract_context_from_request(req: &HeaderMap) -> Context {
	global::get_text_map_propagator(|propagator| propagator.extract(&HeaderExtractor(req)))
}

pub fn add_context_to_request(req: &mut HeaderMap, ctx: &Context) {
	global::get_text_map_propagator(|propagator| {
		propagator.inject_context(ctx, &mut HeaderInjector(req))
	});
	req.insert("baggage", "is_synthetic=true".parse().unwrap());
}

fn get_resource() -> Resource {
	static RESOURCE: OnceLock<Resource> = OnceLock::new();
	RESOURCE
		.get_or_init(|| Resource::builder().with_service_name("mcp-proxy").build())
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
	pub tracer: Tracer,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Tracer {
	#[serde(rename = "otlp")]
	Otlp { endpoint: Option<String> },
}

pub fn init_tracer(config: Config) -> Result<SdkTracerProvider, ExporterBuildError> {
	let baggage_propagator = BaggagePropagator::new();
	let trace_context_propagator = TraceContextPropagator::new();
	let composite_propagator = TextMapCompositePropagator::new(vec![
		Box::new(baggage_propagator),
		Box::new(trace_context_propagator),
	]);

	info!(cfg=?config, "initializing tracer");
	global::set_text_map_propagator(composite_propagator);
	let builder = SpanExporter::builder();
	let exporter = match config.tracer {
		Tracer::Otlp { endpoint } => {
			let builder = builder.with_tonic();
			match endpoint {
				Some(endpoint) => builder.with_endpoint(endpoint),
				None => builder,
			}
			.build()?
		},
	};

	let provider = SdkTracerProvider::builder()
		.with_span_processor(EnrichWithBaggageSpanProcessor)
		.with_resource(get_resource())
		.with_batch_exporter(exporter)
		.build();

	global::set_tracer_provider(provider.clone());
	Ok(provider)
}
