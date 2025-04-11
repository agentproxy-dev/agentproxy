use crate::metrics::Recorder;
use crate::outbound::backend;
use crate::outbound::openapi;
use crate::outbound::{Target, TargetSpec};
use crate::rbac;
use crate::trcng;
use crate::xds::XdsStore;
use http::HeaderName;
use http::{HeaderMap, HeaderValue, header::AUTHORIZATION};
use itertools::Itertools;
use opentelemetry::trace::Tracer;
use opentelemetry::{Context, trace::SpanKind};
use rmcp::RoleClient;
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::transport::sse::SseTransport;
use rmcp::{
	Error as McpError, RoleServer, ServerHandler, model::CallToolRequestParam, model::Tool, model::*,
	service::RequestContext,
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::instrument;
pub mod metrics;
mod pool;
mod upstream;

lazy_static::lazy_static! {
	static ref DEFAULT_RQ_CTX: RqCtx = RqCtx::default();
}

#[derive(Clone)]
pub struct RqCtx {
	identity: rbac::Identity,
	context: Context,
}

impl Default for RqCtx {
	fn default() -> Self {
		Self {
			identity: rbac::Identity::default(),
			context: Context::new(),
		}
	}
}

impl RqCtx {
	pub fn new(identity: rbac::Identity, context: Context) -> Self {
		Self { identity, context }
	}
}

#[derive(Clone)]
pub struct Relay {
	state: Arc<tokio::sync::RwLock<XdsStore>>,
	pool: Arc<RwLock<pool::ConnectionPool>>,
	metrics: Arc<metrics::Metrics>,
}

impl Relay {
	pub fn new(state: Arc<tokio::sync::RwLock<XdsStore>>, metrics: Arc<metrics::Metrics>) -> Self {
		Self {
			state: state.clone(),
			pool: Arc::new(RwLock::new(pool::ConnectionPool::new(state.clone()))),
			metrics,
		}
	}
}

impl Relay {
	pub async fn remove_target(&self, name: &str) -> Result<(), tokio::task::JoinError> {
		tracing::info!("removing target: {}", name);
		let mut pool = self.pool.write().await;
		match pool.remove(name).await {
			Some(target_arc) => {
				// Try this a few times?
				let target = Arc::into_inner(target_arc).unwrap();
				match target {
					upstream::UpstreamTarget::Mcp(m) => {
						m.cancel().await?;
					},
					_ => {
						todo!()
					},
				}
				Ok(())
			},
			None => Ok(()),
		}
	}
}

// TODO: lists and gets can be macros
impl ServerHandler for Relay {
	#[instrument(level = "debug", skip_all)]
	fn get_info(&self) -> ServerInfo {
		ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                completions: None,
                experimental: None,
                logging: None,
                prompts: Some(PromptsCapability::default()),
                resources: Some(ResourcesCapability::default()),
                tools: Some(ToolsCapability {
                    list_changed: None,
                }),
            },
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides a counter tool that can increment and decrement values. The counter starts at 0 and can be modified using the 'increment' and 'decrement' tools. Use 'get_value' to check the current count.".to_string(),
            ),
        }
	}

	#[instrument(level = "debug", skip_all)]
	async fn list_resources(
		&self,
		request: Option<PaginatedRequestParam>,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<ListResourcesResult, McpError> {
		let rq_ctx = _context
			.extensions
			.get::<RqCtx>()
			.unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::get_tracer()
			.span_builder("list_resources")
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		let all = connections.into_iter().map(|(_name, svc)| {
			let request = request.clone();
			async move {
				match svc.list_resources(request).await {
					Ok(r) => Ok(r.resources),
					Err(e) => Err(e),
				}
			}
		});

		// TODO: Handle errors
		let (results, _errors): (Vec<_>, Vec<_>) = futures::future::join_all(all)
			.await
			.into_iter()
			.partition_result();

		Ok(ListResourcesResult {
			resources: results.into_iter().flatten().collect(),
			next_cursor: None,
		})
	}

	#[instrument(level = "debug", skip_all)]
	async fn list_resource_templates(
		&self,
		request: Option<PaginatedRequestParam>,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<ListResourceTemplatesResult, McpError> {
		let rq_ctx = _context
			.extensions
			.get::<RqCtx>()
			.unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::get_tracer()
			.span_builder("list_resource_templates")
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		let all = connections.into_iter().map(|(_name, svc)| {
			let request = request.clone();
			async move {
				match svc.list_resource_templates(request).await {
					Ok(r) => Ok(r.resource_templates),
					Err(e) => Err(e),
				}
			}
		});

		let (results, _errors): (Vec<_>, Vec<_>) = futures::future::join_all(all)
			.await
			.into_iter()
			.partition_result();

		self.metrics.clone().record(
			&metrics::ListCall {
				resource_type: "resource_template".to_string(),
			},
			(),
		);

		Ok(ListResourceTemplatesResult {
			resource_templates: results.into_iter().flatten().collect(),
			next_cursor: None,
		})
	}

	#[instrument(level = "debug", skip_all)]
	async fn list_prompts(
		&self,
		request: Option<PaginatedRequestParam>,
		context: RequestContext<RoleServer>,
	) -> std::result::Result<ListPromptsResult, McpError> {
		let rq_ctx = context.extensions.get::<RqCtx>().unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::get_tracer()
			.span_builder("list_prompts")
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);

		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		let all = connections.into_iter().map(|(_name, svc)| {
			let request = request.clone();
			async move {
				match svc.list_prompts(request).await {
					Ok(r) => Ok(
						r.prompts
							.into_iter()
							.map(|p| Prompt {
								name: format!("{}:{}", _name, p.name),
								description: p.description,
								arguments: p.arguments,
							})
							.collect::<Vec<_>>(),
					),
					Err(e) => Err(e),
				}
			}
		});

		let (results, _errors): (Vec<_>, Vec<_>) = futures::future::join_all(all)
			.await
			.into_iter()
			.partition_result();

		self.metrics.clone().record(
			&metrics::ListCall {
				resource_type: "prompt".to_string(),
			},
			(),
		);
		Ok(ListPromptsResult {
			prompts: results.into_iter().flatten().collect(),
			next_cursor: None,
		})
	}

	#[instrument(
    level = "debug",
    skip_all,
    fields(
        name=%request.uri,
    ),
  )]
	async fn read_resource(
		&self,
		request: ReadResourceRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<ReadResourceResult, McpError> {
		let rq_ctx = _context
			.extensions
			.get::<RqCtx>()
			.unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::get_tracer()
			.span_builder("read_resource")
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		if !self.state.read().await.policies.validate(
			&rbac::ResourceType::Resource {
				id: request.uri.to_string(),
			},
			&rq_ctx.identity,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}

		let uri = request.uri.to_string();
		let (service_name, resource) = uri.split_once(':').unwrap();
		let mut pool = self.pool.write().await;
		let service_arc = pool
			.get_or_create(rq_ctx, service_name)
			.await
			.map_err(|_e| {
				McpError::invalid_request(format!("Service {} not found", service_name), None)
			})?;
		let req = ReadResourceRequestParam {
			uri: resource.to_string(),
		};

		self.metrics.clone().record(
			&metrics::GetResourceCall {
				server: service_name.to_string(),
				uri: resource.to_string(),
			},
			(),
		);
		match service_arc.read_resource(req).await {
			Ok(r) => Ok(r),
			Err(e) => Err(e.into()),
		}
	}

	#[instrument(
    level = "debug",
    skip_all,
    fields(
        name=%request.name,
    ),
  )]
	async fn get_prompt(
		&self,
		request: GetPromptRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<GetPromptResult, McpError> {
		let rq_ctx = _context
			.extensions
			.get::<RqCtx>()
			.unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::get_tracer()
			.span_builder("get_prompt")
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		if !self.state.read().await.policies.validate(
			&rbac::ResourceType::Prompt {
				id: request.name.to_string(),
			},
			&rq_ctx.identity,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}

		let prompt_name = request.name.to_string();
		let (service_name, prompt) = prompt_name.split_once(':').unwrap();
		let mut pool = self.pool.write().await;
		let svc = pool
			.get_or_create(rq_ctx, service_name)
			.await
			.map_err(|_e| {
				McpError::invalid_request(format!("Service {} not found", service_name), None)
			})?;
		let req = GetPromptRequestParam {
			name: prompt.to_string(),
			arguments: request.arguments,
		};

		self.metrics.clone().record(
			&metrics::GetPromptCall {
				server: service_name.to_string(),
				name: prompt.to_string(),
			},
			(),
		);
		match svc.get_prompt(req).await {
			Ok(r) => Ok(r),
			Err(e) => Err(e.into()),
		}
	}

	#[instrument(level = "debug", skip_all)]
	async fn list_tools(
		&self,
		request: Option<PaginatedRequestParam>,
		context: RequestContext<RoleServer>,
	) -> std::result::Result<ListToolsResult, McpError> {
		let rq_ctx = context.extensions.get::<RqCtx>().unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::get_tracer()
			.span_builder("list_tools")
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		let all = connections.into_iter().map(|(_name, svc_arc)| {
			let request = request.clone();
			async move {
				match svc_arc.list_tools(request).await {
					Ok(r) => Ok(
						r.tools
							.into_iter()
							.map(|t| Tool {
								annotations: None,
								name: Cow::Owned(format!("{}:{}", _name, t.name)),
								description: t.description,
								input_schema: t.input_schema,
							})
							.collect::<Vec<_>>(),
					),
					Err(e) => Err(e),
				}
			}
		});

		let (results, _errors): (Vec<_>, Vec<_>) = futures::future::join_all(all)
			.await
			.into_iter()
			.partition_result();

		self.metrics.clone().record(
			&metrics::ListCall {
				resource_type: "tool".to_string(),
			},
			(),
		);

		Ok(ListToolsResult {
			tools: results.into_iter().flatten().collect(),
			next_cursor: None,
		})
	}

	#[instrument(
    level = "debug",
    skip_all,
    fields(
        name=%request.name,
    ),
  )]
	async fn call_tool(
		&self,
		request: CallToolRequestParam,
		context: RequestContext<RoleServer>,
	) -> std::result::Result<CallToolResult, McpError> {
		let rq_ctx = context.extensions.get::<RqCtx>().unwrap_or(&DEFAULT_RQ_CTX);
		let span_context: &Context = &rq_ctx.context;
		let tracer = trcng::get_tracer();
		let _span = trcng::get_tracer()
			.span_builder("call_tool")
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, span_context);
		if !self.state.read().await.policies.validate(
			&rbac::ResourceType::Tool {
				id: request.name.to_string(),
			},
			&rq_ctx.identity,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}
		let tool_name = request.name.to_string();
		let (service_name, tool) = tool_name
			.split_once(':')
			.ok_or(McpError::invalid_request("invalid tool name", None))?;
		let mut pool = self.pool.write().await;
		let svc = pool
			.get_or_create(rq_ctx, service_name)
			.await
			.map_err(|_e| {
				McpError::invalid_request(format!("Service {} not found", service_name), None)
			})?;
		let req = CallToolRequestParam {
			name: Cow::Owned(tool.to_string()),
			arguments: request.arguments,
		};

		self.metrics.clone().record(
			&metrics::ToolCall {
				server: service_name.to_string(),
				name: tool.to_string(),
			},
			(),
		);

		match svc.call_tool(req).await {
			Ok(r) => Ok(r),
			Err(e) => {
				self.metrics.clone().record(
					&metrics::ToolCallError {
						server: service_name.to_string(),
						name: tool.to_string(),
						error_type: e.error_code(),
					},
					(),
				);
				Err(e.into())
			},
		}
	}
}
