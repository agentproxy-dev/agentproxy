use std::fmt::Debug;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};
use std::time::{Instant, SystemTime};

use crossbeam::atomic::AtomicCell;
use http_body::{Body, Frame, SizeHint};
use tracing::event;

use crate::telemetry::metrics::{CommonTrafficLabels, Metrics};
use crate::telemetry::trc;
use crate::transport::stream::{TCPConnectionInfo, TLSConnectionInfo};
use crate::types::agent::{
	BackendName, GatewayName, ListenerName, RouteName, RouteRuleName, Target,
};
use crate::types::discovery::NamespacedHostname;
use crate::{llm, mcp};

/// AsyncLog is a wrapper around an item that can be atomically set.
/// The intent is to provide additional info to the log after we have lost the RequestLog reference,
/// generally for things that rely on the response body.
#[derive(Clone)]
pub struct AsyncLog<T>(Arc<AtomicCell<Option<T>>>);

impl<T> AsyncLog<T> {
	// non_atomic_mutate is a racey method to modify the current value.
	// If there is no current value, a default is used.
	// This is NOT atomically safe; during the mutation, loads() on the item will be empty.
	// This is ok for our usage cases
	pub fn non_atomic_mutate(&self, f: impl FnOnce(&mut T)) {
		let Some(mut cur) = self.0.take() else {
			return;
		};
		f(&mut cur);
		self.0.store(Some(cur));
	}
}

impl<T> AsyncLog<T> {
	pub fn store(&self, v: Option<T>) {
		self.0.store(v)
	}
	pub fn take(&self) -> Option<T> {
		self.0.take()
	}
}

impl<T: Copy> AsyncLog<T> {
	pub fn load(&self) -> Option<T> {
		self.0.load()
	}
}

impl<T> Default for AsyncLog<T> {
	fn default() -> Self {
		AsyncLog(Arc::new(AtomicCell::new(None)))
	}
}

impl<T: Debug> Debug for AsyncLog<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AsyncLog").finish_non_exhaustive()
	}
}

#[derive(Default, Debug)]
pub struct RequestLog {
	pub tracer: Option<trc::Tracer>,
	pub metrics: Option<Arc<Metrics>>,

	pub start: Option<Instant>,
	pub tcp_info: Option<TCPConnectionInfo>,
	pub tls_info: Option<TLSConnectionInfo>,

	pub endpoint: Option<Target>,

	pub gateway_name: Option<GatewayName>,
	pub listener_name: Option<ListenerName>,
	pub route_rule_name: Option<RouteRuleName>,
	pub route_name: Option<RouteName>,
	pub backend_name: Option<BackendName>,

	pub host: Option<String>,
	pub method: Option<::http::Method>,
	pub path: Option<String>,
	pub version: Option<::http::Version>,
	pub status: Option<crate::http::StatusCode>,

	pub jwt_sub: Option<String>,

	pub retry_attempt: Option<u8>,
	pub error: Option<String>,

	pub grpc_status: AsyncLog<u8>,
	pub mcp_status: AsyncLog<mcp::sse::MCPInfo>,

	pub incoming_span: Option<trc::TraceParent>,
	pub outgoing_span: Option<trc::TraceParent>,

	pub llm_request: Option<llm::LLMRequest>,
	pub llm_response: AsyncLog<llm::LLMResponse>,

	pub a2a_method: Option<&'static str>,

	pub inference_pool: Option<SocketAddr>,
}

impl Drop for RequestLog {
	fn drop(&mut self) {
		let tcp_info = self.tcp_info.as_ref().expect("tODO");

		let dur = format!("{}ms", self.start.unwrap().elapsed().as_millis());
		let grpc = self.grpc_status.load();
		if let Some(t) = &self.tracer {
			t.send(self)
		};
		if let Some(m) = &self.metrics {
			m.requests
				.get_or_create(&CommonTrafficLabels {
					gateway: (&self.gateway_name).into(),
					listener: (&self.listener_name).into(),
					route: (&self.route_name).into(),
					route_rule: (&self.route_rule_name).into(),
					backend: (&self.backend_name).into(),
					method: self.method.clone().into(),
					status: self.status.as_ref().map(|s| s.as_u16()).into(),
				})
				.inc();
		}

		let llm_response = self.llm_response.take();
		if let (Some(req), Some(resp)) = (self.llm_request.as_ref(), llm_response.as_ref()) {
			if Some(req.input_tokens) != resp.input_tokens_from_response {
				// TODO: remove this, just for dev
				tracing::warn!("maybe bug: mismatch in tokens {req:?}, {resp:?}");
			}
		}
		let input_tokens = llm_response
			.as_ref()
			.and_then(|t| t.input_tokens_from_response)
			.or_else(|| self.llm_request.as_ref().map(|req| req.input_tokens));

		let mcp = self.mcp_status.take();

		event!(
			target: "request",
			parent: None,
			tracing::Level::INFO,

			gateway = self.gateway_name.as_ref().map(display),
			listener = self.listener_name.as_ref().map(display),
			route_rule = self.route_rule_name.as_ref().map(display),
			route = self.route_name.as_ref().map(display),

			endpoint = self.endpoint.as_ref().map(display),

			src.addr = %tcp_info.peer_addr,

			http.method = self.method.as_ref().map(display),
			http.host = self.host.as_ref().map(display),
			http.path = self.path.as_ref().map(display),
			// TODO: incoming vs outgoing
			http.version = self.version.as_ref().map(debug),
			http.status = self.status.as_ref().map(|s| s.as_u16()),
			grpc.status = grpc,

			trace.id = self.outgoing_span.as_ref().map(|id| display(id.trace_id())),
			span.id = self.outgoing_span.as_ref().map(|id| display(id.span_id())),

			jwt.sub = self.jwt_sub,

			a2a.method = self.a2a_method.as_ref().map(display),

			mcp.target = mcp.as_ref().and_then(|m| m.target_name.as_ref()).map(display),
			mcp.tool = mcp.as_ref().and_then(|m| m.tool_call_name.as_ref()).map(display),

			inferencepool.selected_endpoint = self.inference_pool.as_ref().map(display),

			llm.provider = self.llm_request.as_ref().map(|l| display(&l.provider)),
			llm.request.model = self.llm_request.as_ref().map(|l| display(&l.request_model)),
			llm.request.tokens = input_tokens.map(display),
			llm.response.model = llm_response.as_ref().and_then(|l| l.provider_model.clone()).map(display),
			llm.response.tokens = llm_response.as_ref().and_then(|l| l.output_tokens).map(display),

			retry.attempt = self.retry_attempt.as_ref(),
			error = self.error.as_ref().map(ToString::to_string),

			duration = %dur,
		);
	}
}

fn to_value<T: AsRef<str>>(t: &T) -> impl tracing::Value + '_ {
	let v: &str = t.as_ref();
	v
}

pin_project_lite::pin_project! {
		/// A data stream created from a [`Body`].
		#[derive(Debug)]
		pub struct LogBody<B> {
				#[pin]
				body: B,
				log: RequestLog,
		}
}

impl<B> LogBody<B> {
	/// Create a new `LogBody`
	pub fn new(body: B, log: RequestLog) -> Self {
		Self { body, log }
	}
}

impl<B: Body + Debug> Body for LogBody<B>
where
	B::Data: Debug,
{
	type Data = B::Data;
	type Error = B::Error;

	fn poll_frame(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
		let this = self.project();
		let result = ready!(this.body.poll_frame(cx));
		match result {
			Some(Ok(frame)) => {
				if let Some(trailer) = frame.trailers_ref() {
					crate::proxy::httpproxy::maybe_set_grpc_status(&this.log.grpc_status, trailer);
				}
				Poll::Ready(Some(Ok(frame)))
			},
			res => Poll::Ready(res),
		}
	}

	fn is_end_stream(&self) -> bool {
		self.body.is_end_stream()
	}

	fn size_hint(&self) -> SizeHint {
		self.body.size_hint()
	}
}
