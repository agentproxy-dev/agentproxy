use crate::mtrcs::Recorder;
use crate::outbound::McpTargetSpec;
use crate::outbound::backend;
use crate::outbound::openapi;
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
use std::collections::BTreeSet;
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

const DELIMITER: &str = "_";

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
	pool: Arc<RwLock<pool::ConnectionPool>>,
	metrics: Arc<metrics::Metrics>,
	policies: rbac::RuleSets,
	initialized: Arc<RwLock<BTreeSet<String>>>,
}

impl Relay {
	pub fn new(
		state: Arc<tokio::sync::RwLock<XdsStore>>,
		metrics: Arc<metrics::Metrics>,
		policies: rbac::RuleSets,
		listener_name: String,
	) -> Self {
		Self {
			pool: Arc::new(RwLock::new(pool::ConnectionPool::new(
				state.clone(),
				listener_name,
			))),
			metrics,
			policies,
			initialized: Arc::new(RwLock::new(BTreeSet::new())),
		}
	}
}

impl Relay {
	pub async fn remove_target(&self, name: &str) -> Result<(), tokio::task::JoinError> {
		tracing::info!("removing target: {}", name);
		let mut pool = self.pool.write().await;
		match pool.remove(name).await {
			Some(target) => {
				match target {
					upstream::UpstreamTarget::Mcp(m) => {
						m.cancel().await?;
					},
					_ => {
						// Nothing to do here
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

	async fn initialize(
		&self,
		request: InitializeRequestParam,
		context: RequestContext<RoleServer>,
	) -> Result<InitializeResult, McpError> {
		let rq_ctx = context.extensions.get::<RqCtx>().unwrap_or(&DEFAULT_RQ_CTX);
		let mut initialized = self.initialized.write().await;

		// List servers and initialize the ones that are not initialized
		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx, &context.peer)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		for (name, upstream) in connections {
			if !initialized.contains(&name) {
				upstream.initialize(request.clone()).await?;
				initialized.insert(name.clone());
			}
		}
		Ok(InitializeResult::default())
	}

	#[instrument(level = "debug", skip_all)]
	async fn list_resources(
		&self,
		request: Option<PaginatedRequestParam>,
		context: RequestContext<RoleServer>,
	) -> std::result::Result<ListResourcesResult, McpError> {
		let rq_ctx = context.extensions.get::<RqCtx>().unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::start_span("list_resources", &rq_ctx.identity)
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx, &context.peer)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		let all = connections.into_iter().map(|(_name, svc)| {
			let request = request.clone();
			async move {
				match svc.list_resources(request, rq_ctx).await {
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
		let _span = trcng::start_span("list_resource_templates", &rq_ctx.identity)
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx, &_context.peer)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		let all = connections.into_iter().map(|(_name, svc)| {
			let request = request.clone();
			async move {
				match svc.list_resource_templates(request, rq_ctx).await {
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
			metrics::ListCall {
				resource_type: "resource_template".to_string(),
				params: vec![],
			},
			&rq_ctx.identity,
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
		let _span = trcng::start_span("list_prompts", &rq_ctx.identity)
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);

		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx, &context.peer)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		let all = connections.into_iter().map(|(_name, svc)| {
			let request = request.clone();
			async move {
				match svc.list_prompts(request, rq_ctx).await {
					Ok(r) => Ok(
						r.prompts
							.into_iter()
							.map(|p| Prompt {
								name: format!("{}{}{}", _name, DELIMITER, p.name),
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

		self.metrics.record(
			metrics::ListCall {
				resource_type: "prompt".to_string(),
				params: vec![],
			},
			&rq_ctx.identity,
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
		context: RequestContext<RoleServer>,
	) -> std::result::Result<ReadResourceResult, McpError> {
		let rq_ctx = context.extensions.get::<RqCtx>().unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::start_span("read_resource", &rq_ctx.identity)
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		let uri = request.uri.to_string();
		let (service_name, resource) = uri
			.split_once(DELIMITER)
			.ok_or(McpError::invalid_request("invalid resource name", None))?;
		if !self.policies.validate(
			&rbac::ResourceType::Resource(rbac::ResourceId::new(
				service_name.to_string(),
				resource.to_string(),
			)),
			&rq_ctx.identity,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}

		let mut pool = self.pool.write().await;
		let service_arc = pool
			.get_or_create(rq_ctx, &context.peer, service_name)
			.await
			.map_err(|_e| {
				McpError::invalid_request(format!("Service {} not found", service_name), None)
			})?;
		let req = ReadResourceRequestParam {
			uri: resource.to_string(),
		};

		self.metrics.clone().record(
			metrics::GetResourceCall {
				server: service_name.to_string(),
				uri: resource.to_string(),
				params: vec![],
			},
			&rq_ctx.identity,
		);
		match service_arc.read_resource(req, rq_ctx).await {
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
		context: RequestContext<RoleServer>,
	) -> std::result::Result<GetPromptResult, McpError> {
		let rq_ctx = context.extensions.get::<RqCtx>().unwrap_or(&DEFAULT_RQ_CTX);
		let tracer = trcng::get_tracer();
		let _span = trcng::start_span("get_prompt", &rq_ctx.identity)
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);

		let prompt_name = request.name.to_string();
		let (service_name, prompt) = prompt_name
			.split_once(DELIMITER)
			.ok_or(McpError::invalid_request("invalid prompt name", None))?;
		if !self.policies.validate(
			&rbac::ResourceType::Prompt(rbac::ResourceId::new(
				service_name.to_string(),
				prompt.to_string(),
			)),
			&rq_ctx.identity,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}
		let mut pool = self.pool.write().await;
		let svc = pool
			.get_or_create(rq_ctx, &context.peer, service_name)
			.await
			.map_err(|_e| {
				McpError::invalid_request(format!("Service {} not found", service_name), None)
			})?;
		let req = GetPromptRequestParam {
			name: prompt.to_string(),
			arguments: request.arguments,
		};

		self.metrics.clone().record(
			metrics::GetPromptCall {
				server: service_name.to_string(),
				name: prompt.to_string(),
				params: vec![],
			},
			&rq_ctx.identity,
		);
		match svc.get_prompt(req, rq_ctx).await {
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
		let _span = trcng::start_span("list_tools", &rq_ctx.identity)
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, &rq_ctx.context);
		let mut pool = self.pool.write().await;
		let connections = pool
			.list(rq_ctx, &context.peer)
			.await
			.map_err(|e| McpError::internal_error(format!("Failed to list connections: {}", e), None))?;
		let all = connections.into_iter().map(|(_name, svc_arc)| {
			let request = request.clone();
			async move {
				match svc_arc.list_tools(request, rq_ctx).await {
					Ok(r) => Ok(
						r.tools
							.into_iter()
							.map(|t| Tool {
								annotations: None,
								name: Cow::Owned(format!("{}{}{}", _name, DELIMITER, t.name)),
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
			metrics::ListCall {
				resource_type: "tool".to_string(),
				params: vec![],
			},
			&rq_ctx.identity,
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
		let _span = trcng::start_span("call_tool", &rq_ctx.identity)
			.with_kind(SpanKind::Server)
			.start_with_context(tracer, span_context);
		let tool_name = request.name.to_string();
		let (service_name, tool) = tool_name
			.split_once(DELIMITER)
			.ok_or(McpError::invalid_request("invalid tool name", None))?;
		if !self.policies.validate(
			&rbac::ResourceType::Tool(rbac::ResourceId::new(
				service_name.to_string(),
				tool.to_string(),
			)),
			&rq_ctx.identity,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}
		let mut pool = self.pool.write().await;
		let svc = pool
			.get_or_create(rq_ctx, &context.peer, service_name)
			.await
			.map_err(|_e| {
				McpError::invalid_request(format!("Service {} not found", service_name), None)
			})?;
		let req = CallToolRequestParam {
			name: Cow::Owned(tool.to_string()),
			arguments: request.arguments,
		};

		self.metrics.record(
			metrics::ToolCall {
				server: service_name.to_string(),
				name: tool.to_string(),
				params: vec![],
			},
			&rq_ctx.identity,
		);

		match svc.call_tool(req, rq_ctx).await {
			Ok(r) => Ok(r),
			Err(e) => {
				self.metrics.record(
					metrics::ToolCallError {
						server: service_name.to_string(),
						name: tool.to_string(),
						error_type: e.error_code(),
						params: vec![],
					},
					&rq_ctx.identity,
				);
				Err(e.into())
			},
		}
	}
}
