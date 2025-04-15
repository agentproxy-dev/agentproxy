use axum::{Router, extract::State, http::StatusCode, routing::get};
use std::collections::HashMap;
use std::{mem, sync::Arc};

use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use serde::{Deserialize, Serialize};
use tracing::error;

/// Creates a metrics sub registry for mcp-proxy.
pub fn sub_registry(registry: &mut Registry) -> &mut Registry {
	registry.sub_registry_with_prefix("mcp-proxy")
}

pub struct Deferred<'a, F, T>
where
	F: FnOnce(&'a T),
	T: ?Sized,
{
	param: &'a T,
	deferred_fn: Option<F>,
}

impl<'a, F, T> Deferred<'a, F, T>
where
	F: FnOnce(&'a T),
	T: ?Sized,
{
	pub fn new(param: &'a T, deferred_fn: F) -> Self {
		Self {
			param,
			deferred_fn: Some(deferred_fn),
		}
	}
}

impl<'a, F, T> Drop for Deferred<'a, F, T>
where
	F: FnOnce(&'a T),
	T: ?Sized,
{
	fn drop(&mut self) {
		if let Some(deferred_fn) = mem::take(&mut self.deferred_fn) {
			(deferred_fn)(self.param);
		} else {
			error!("defer deferred record failed, event is gone");
		}
	}
}

pub trait DeferRecorder {
	#[must_use = "metric will be dropped (and thus recorded) immediately if not assigned"]
	/// Perform a record operation on this object when the returned [Deferred] object is
	/// dropped.
	fn defer_record<'a, F>(&'a self, record: F) -> Deferred<'a, F, Self>
	where
		F: FnOnce(&'a Self),
	{
		Deferred::new(self, record)
	}
}

pub trait Recorder<E, T> {
	/// Record the given event
	fn record(&self, event: E, meta: T);
}

pub trait IncrementRecorder<E>: Recorder<E, u64> {
	/// Record the given event by incrementing the counter by count
	fn increment(&self, event: E);
}

impl<E, R> IncrementRecorder<E> for R
where
	R: Recorder<E, u64>,
{
	fn increment(&self, event: E) {
		self.record(event, 1);
	}
}

#[derive(Clone, Default)]
struct App {
	registry: Arc<Registry>,
}

impl App {
	fn new(registry: Arc<Registry>) -> Self {
		Self { registry }
	}
	fn router(&self) -> Router {
		Router::new()
			.route("/metrics", get(metrics_handler))
			.with_state(self.clone())
	}
}

async fn metrics_handler(State(app): State<App>) -> Result<String, StatusCode> {
	let mut buffer = String::new();
	match encode(&mut buffer, &app.registry) {
		Ok(_) => Ok(buffer),
		Err(e) => {
			error!("error encoding metrics: {:?}", e);
			Err(StatusCode::INTERNAL_SERVER_ERROR)
		},
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_host")]
	pub host: String,
	#[serde(default = "default_port")]
	pub port: u16,
	#[serde(default)]
	pub tags: HashMap<String, String>,
}

fn default_host() -> String {
	"127.0.0.1".to_string()
}

fn default_port() -> u16 {
	9091
}

impl Default for Config {
	fn default() -> Self {
		Self {
			host: default_host(),
			port: default_port(),
			tags: HashMap::new(),
		}
	}
}

pub async fn start(
	registry: Arc<Registry>,
	ct: tokio_util::sync::CancellationToken,
	cfg: Option<Config>,
) -> Result<(), std::io::Error> {
	tracing::error!("howardjohn: START METRICS {cfg:?}");
	let cfg = cfg.unwrap_or_default();
	let listener = tokio::net::TcpListener::bind(format!("{}:{}", cfg.host, cfg.port)).await?;
	let app = App::new(registry);
	let router = app.router();
	axum::serve(listener, router)
		.with_graceful_shutdown(async move {
			ct.cancelled().await;
		})
		.await
}
