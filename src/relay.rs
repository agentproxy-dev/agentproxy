use crate::rbac;
use crate::xds::{OpenAPISchema, Target, TargetSpec, XdsStore};
use http::Method;
use itertools::Itertools;
use rmcp::RoleClient;
use rmcp::serve_client;
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::transport::sse::SseTransport;
use rmcp::{
	Error as McpError, RoleServer, ServerHandler, model::CallToolRequestParam, model::Tool, model::*,
	service::RequestContext,
};
use serde_json::json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Relay {
	state: Arc<std::sync::RwLock<XdsStore>>,
	pool: Arc<RwLock<ConnectionPool>>,
	id: rbac::Identity,
}

impl Relay {
	pub fn new(state: Arc<std::sync::RwLock<XdsStore>>, id: rbac::Identity) -> Self {
		Self {
			state: state.clone(),
			pool: Arc::new(RwLock::new(ConnectionPool::new(state.clone()))),
			id,
		}
	}
}

// TODO: lists and gets can be macros
impl ServerHandler for Relay {
	fn get_info(&self) -> ServerInfo {
		ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
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

	async fn list_resources(
		&self,
		request: PaginatedRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<ListResourcesResult, McpError> {
		let pool = self.pool.read().await;
		let all = pool.iter().await.map(|(_name, svc)| {
			let svc = svc.clone();
			let request = request.clone();
			async move {
				match svc.as_ref().read().await.list_resources(request).await {
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

	async fn read_resource(
		&self,
		request: ReadResourceRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<ReadResourceResult, McpError> {
		if !self.state.read().unwrap().policies.validate(
			&rbac::ResourceType::Resource {
				id: request.uri.to_string(),
			},
			&self.id,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}
		let pool = self.pool.read().await;
		let target = pool.get(&request.uri).await.unwrap();
		let result = target.as_ref().read().await.read_resource(request).await?;

		Ok(ReadResourceResult {
			contents: result.contents,
		})
	}

	async fn list_resource_templates(
		&self,
		request: PaginatedRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<ListResourceTemplatesResult, McpError> {
		let pool = self.pool.read().await;
		let all = pool.iter().await.map(|(_name, svc)| {
			let svc = svc.clone();
			let request = request.clone();
			async move {
				match svc
					.as_ref()
					.read()
					.await
					.list_resource_templates(request)
					.await
				{
					Ok(r) => Ok(r.resource_templates),
					Err(e) => Err(e),
				}
			}
		});

		let (results, _errors): (Vec<_>, Vec<_>) = futures::future::join_all(all)
			.await
			.into_iter()
			.partition_result();

		Ok(ListResourceTemplatesResult {
			resource_templates: results.into_iter().flatten().collect(),
			next_cursor: None,
		})
	}

	async fn list_prompts(
		&self,
		request: PaginatedRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<ListPromptsResult, McpError> {
		let pool = self.pool.read().await;
		let all = pool.iter().await.map(|(_name, svc)| {
			let svc = svc.clone();
			let request = request.clone();
			async move {
				match svc.as_ref().read().await.list_prompts(request).await {
					Ok(r) => Ok(r.prompts),
					Err(e) => Err(e),
				}
			}
		});

		let (results, _errors): (Vec<_>, Vec<_>) = futures::future::join_all(all)
			.await
			.into_iter()
			.partition_result();

		Ok(ListPromptsResult {
			prompts: results.into_iter().flatten().collect(),
			next_cursor: None,
		})
	}

	async fn get_prompt(
		&self,
		request: GetPromptRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<GetPromptResult, McpError> {
		if !self.state.read().unwrap().policies.validate(
			&rbac::ResourceType::Prompt {
				id: request.name.to_string(),
			},
			&self.id,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}
		let tool_name = request.name.to_string();
		let (service_name, tool) = tool_name.split_once(':').unwrap();
		let pool = self.pool.read().await;
		let service = pool.get(service_name).await.unwrap();
		let req = GetPromptRequestParam {
			name: tool.to_string(),
			arguments: request.arguments,
		};

		match service.as_ref().read().await.get_prompt(req).await {
			Ok(r) => Ok(r),
			Err(e) => Err(e.into()),
		}
	}

	async fn list_tools(
		&self,
		request: PaginatedRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<ListToolsResult, McpError> {
		// TODO: Use iterators
		// TODO: Handle individual errors
		// TODO: Do we want to handle pagination here, or just pass it through?
		tracing::trace!("listing tools");
		let pool = self.pool.read().await;
		let all = pool.iter().await.map(|(_name, svc)| {
			let svc = svc.clone();
			let request = request.clone();
			async move {
				match svc.as_ref().read().await.list_tools(request).await {
					Ok(r) => Ok(r.tools),
					Err(e) => Err(e),
				}
			}
		});

		let (results, _errors): (Vec<_>, Vec<_>) = futures::future::join_all(all)
			.await
			.into_iter()
			.partition_result();

		Ok(ListToolsResult {
			tools: results.into_iter().flatten().collect(),
			next_cursor: None,
		})
	}

	async fn call_tool(
		&self,
		request: CallToolRequestParam,
		_context: RequestContext<RoleServer>,
	) -> std::result::Result<CallToolResult, McpError> {
		tracing::trace!("calling tool: {:?}", request);
		if !self.state.read().unwrap().policies.validate(
			&rbac::ResourceType::Tool {
				id: request.name.to_string(),
			},
			&self.id,
		) {
			return Err(McpError::invalid_request("not allowed", None));
		}
		let tool_name = request.name.to_string();
		let (service_name, tool) = tool_name.split_once(':').unwrap();
		let pool = self.pool.read().await;
		let service = pool.get(service_name).await.unwrap();
		let req = CallToolRequestParam {
			name: Cow::Owned(tool.to_string()),
			arguments: request.arguments,
		};

		match service.as_ref().read().await.call_tool(req).await {
			Ok(r) => Ok(r),
			Err(e) => Err(e.into()),
		}
	}
}

#[derive(Clone)]
struct ConnectionPool {
	state: Arc<std::sync::RwLock<XdsStore>>,

	by_name: Arc<RwLock<HashMap<String, Arc<RwLock<UpstreamTarget>>>>>,
}

impl ConnectionPool {
	fn new(state: Arc<std::sync::RwLock<XdsStore>>) -> Self {
		Self {
			state,
			by_name: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	async fn get(&self, name: &str) -> Option<Arc<RwLock<UpstreamTarget>>> {
		tracing::trace!("getting connection for target: {}", name);
		let by_name = self.by_name.read().await;
		match by_name.get(name) {
			Some(connection) => {
				tracing::trace!("connection found for target: {}", name);
				Some(connection.clone())
			},
			None => {
				let target = { self.state.read().unwrap().targets.get(name).cloned() };
				match target {
					Some(target) => {
						// We want write access to the by_name map, so we drop the read lock
						// TODO: Fix this
						drop(by_name);
						match self.connect(&target).await {
							Ok(connection) => Some(connection),
							Err(e) => {
								tracing::error!("Error connecting to target: {}", e);
								None
							},
						}
					},
					None => {
						tracing::error!("Target not found: {}", name);
						// Need to demand it, but this should never happen
						None
					},
				}
			},
		}
	}

	async fn iter(&self) -> impl Iterator<Item = (String, Arc<RwLock<UpstreamTarget>>)> {
		// Iterate through all state targets, and get the connection from the pool
		// If the connection is not in the pool, connect to it and add it to the pool
		let targets: Vec<(String, Target)> = {
			let state = self.state.read().unwrap();
			state
				.targets
				.iter()
				.map(|(name, target)| (name.clone(), target.clone()))
				.collect()
		};
		let x = targets.iter().map(|(name, _target)| async move {
			let connection = self.get(name).await.unwrap();
			(name.clone(), connection)
		});

		let x = futures::future::join_all(x).await;
		x.into_iter()
	}

	async fn connect(&self, target: &Target) -> Result<Arc<RwLock<UpstreamTarget>>, anyhow::Error> {
		tracing::trace!("connecting to target: {}", target.name);
		let transport: UpstreamTarget = match &target.spec {
			TargetSpec::Sse { host, port } => {
				tracing::trace!("starting sse transport for target: {}", target.name);
				let transport: SseTransport =
					SseTransport::start(format!("http://{}:{}/sse", host, port).as_str()).await?;
				UpstreamTarget::Mcp(serve_client((), transport).await?)
			},
			TargetSpec::Stdio { cmd, args } => {
				tracing::trace!("starting stdio transport for target: {}", target.name);
				UpstreamTarget::Mcp(
					serve_client(
						(),
						TokioChildProcess::new(Command::new(cmd).args(args)).unwrap(),
					)
					.await?,
				)
			},
			TargetSpec::OpenAPI { host, port, schema } => {
				tracing::info!("starting OpenAPI transport for target: {}", target.name);
				let client = reqwest::Client::new();

				UpstreamTarget::OpenAPI(OpenAPIHandler {
					host: format!("http://{}:{}", host, port),
					client,
					schema: schema.clone(),
				})
			},
		};
		let connection = Arc::new(RwLock::new(transport));
		// We need to drop this lock quick
		let mut by_name = self.by_name.write().await;
		by_name.insert(target.name.clone(), connection.clone());
		Ok(connection)
	}
}

/// UpstreamTarget defines a source for MCP information.
#[derive(Debug)]
enum UpstreamTarget {
	Mcp(RunningService<RoleClient, ()>),
	OpenAPI(OpenAPIHandler),
}

enum UpstreamError {
	ServiceError(rmcp::ServiceError),
	OpenAPIError(anyhow::Error),
}

impl From<rmcp::ServiceError> for UpstreamError {
	fn from(value: rmcp::ServiceError) -> Self {
		UpstreamError::ServiceError(value)
	}
}

impl From<anyhow::Error> for UpstreamError {
	fn from(value: anyhow::Error) -> Self {
		UpstreamError::OpenAPIError(value)
	}
}

impl From<UpstreamError> for ErrorData {
	fn from(value: UpstreamError) -> Self {
		match value {
			UpstreamError::OpenAPIError(e) => ErrorData::internal_error(e.to_string(), None),
			UpstreamError::ServiceError(e) => match e {
				rmcp::ServiceError::McpError(e) => e,
				rmcp::ServiceError::Timeout { timeout } => {
					ErrorData::internal_error(format!("request timed out after {:?}", timeout), None)
				},
				rmcp::ServiceError::Cancelled { reason } => match reason {
					Some(reason) => ErrorData::internal_error(reason.clone(), None),
					None => ErrorData::internal_error("unknown reason", None),
				},
				rmcp::ServiceError::UnexpectedResponse => {
					ErrorData::internal_error("unexpected response", None)
				},
				rmcp::ServiceError::Transport(e) => ErrorData::internal_error(e.to_string(), None),
				_ => ErrorData::internal_error("unknown error", None),
			},
		}
	}
}

impl UpstreamTarget {
	async fn list_tools(
		&self,
		request: PaginatedRequestParam,
	) -> Result<ListToolsResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => Ok(m.list_tools(request).await?),
			UpstreamTarget::OpenAPI(m) => Ok(ListToolsResult {
				next_cursor: None,
				tools: m.tools(),
			}),
		}
	}

	async fn get_prompt(
		&self,
		request: GetPromptRequestParam,
	) -> Result<GetPromptResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => Ok(m.get_prompt(request).await?),
			UpstreamTarget::OpenAPI(_) => Ok(GetPromptResult {
				description: None,
				messages: vec![],
			}),
		}
	}

	async fn list_prompts(
		&self,
		request: PaginatedRequestParam,
	) -> Result<ListPromptsResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => Ok(m.list_prompts(request).await?),
			UpstreamTarget::OpenAPI(_) => Ok(ListPromptsResult {
				next_cursor: None,
				prompts: vec![],
			}),
		}
	}

	async fn list_resources(
		&self,
		request: PaginatedRequestParam,
	) -> Result<ListResourcesResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => Ok(m.list_resources(request).await?),
			UpstreamTarget::OpenAPI(_) => Ok(ListResourcesResult {
				next_cursor: None,
				resources: vec![],
			}),
		}
	}

	async fn list_resource_templates(
		&self,
		request: PaginatedRequestParam,
	) -> Result<ListResourceTemplatesResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => Ok(m.list_resource_templates(request).await?),
			UpstreamTarget::OpenAPI(_) => Ok(ListResourceTemplatesResult {
				next_cursor: None,
				resource_templates: vec![],
			}),
		}
	}

	async fn read_resource(
		&self,
		request: ReadResourceRequestParam,
	) -> Result<ReadResourceResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => Ok(m.read_resource(request).await?),
			UpstreamTarget::OpenAPI(_) => Ok(ReadResourceResult { contents: vec![] }),
		}
	}

	async fn call_tool(
		&self,
		request: CallToolRequestParam,
	) -> Result<CallToolResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => Ok(m.call_tool(request).await?),
			UpstreamTarget::OpenAPI(m) => {
				let res = m
					.call_tool(request.name.as_ref(), request.arguments)
					.await?;
				Ok(CallToolResult {
					content: vec![Content::text(res)],
					is_error: None,
				})
			},
		}
	}
}

#[derive(Debug)]
struct OpenAPIHandler {
	host: String,
	client: reqwest::Client,
	schema: OpenAPISchema,
}

struct UpstreamOpenAPICall {
	method: Method,
	path: String,
	// todo: params
}

impl OpenAPIHandler {
	async fn call_tool(&self, name: &str, args: Option<JsonObject>) -> Result<String, anyhow::Error> {
		let (_, info) = self
			.info()
			.into_iter()
			.find(|(t, _info)| t.name == name)
			.ok_or_else(|| anyhow::anyhow!("tool {} not found", name))?;
		let body = self
			.client
			.request(info.method.clone(), format!("{}{}", self.host, &info.path))
			.json(args.as_ref().unwrap())
			.send()
			.await?
			.text()
			.await?;
		Ok(body)
	}

	fn tools(&self) -> Vec<Tool> {
		self.info().into_iter().map(|(t, _)| t).collect()
	}

	fn info(&self) -> Vec<(Tool, UpstreamOpenAPICall)> {
		self
			.schema
			.paths
			.iter()
			.flat_map(|(path, path_info)| {
				let item = path_info.as_item().unwrap();
				item
					.iter()
					.map(|(method, op)| {
						let name = op.operation_id.clone().expect("TODO");
						let props: Vec<_> = op
							.parameters
							.iter()
							.map(|p| {
								let item = dbg!(p).as_item().unwrap();
								let p = dbg!(item.parameter_data_ref());
								let mut schema = JsonObject::new();
								if let openapiv3::ParameterSchemaOrContent::Schema(openapiv3::ReferenceOr::Item(
									s,
								)) = &p.format
								{
									schema = serde_json::to_value(s)
										.expect("TODO")
										.as_object()
										.expect("TODO")
										.clone();
								}
								if let Some(desc) = &p.description {
									schema.insert("description".to_string(), json!(desc));
								}

								(p.name.clone(), schema, p.required)
							})
							.collect();
						let mut schema = JsonObject::new();
						schema.insert("type".to_string(), json!("object"));
						let required: Vec<String> = props
							.iter()
							.flat_map(|(name, _, req)| if *req { Some(name.clone()) } else { None })
							.collect();
						schema.insert("required".to_string(), json!(required));
						let mut schema_props = JsonObject::new();
						for (name, s, _) in props {
							schema_props.insert(name, json!(s));
						}
						schema.insert("properties".to_string(), json!(schema_props));
						let tool = Tool {
							name: Cow::Owned(name.clone()),
							description: Cow::Owned(
								op.description
									.as_ref()
									.unwrap_or_else(|| op.summary.as_ref().unwrap_or(&name))
									.to_string(),
							),
							// input_schema: Arc::new(Default::default()),
							input_schema: Arc::new(schema),
						};
						let upstream = UpstreamOpenAPICall {
							method: Method::from_bytes(method.as_ref()).expect("todo"),
							path: path.clone(),
						};
						(tool, upstream)
					})
					.collect::<Vec<_>>()
			})
			.collect()
	}
}
