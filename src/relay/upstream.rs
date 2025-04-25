use super::*;

pub(crate) enum UpstreamError {
	ServiceError(rmcp::ServiceError),
	OpenAPIError(anyhow::Error),
}

impl UpstreamError {
	pub(crate) fn error_code(&self) -> String {
		match self {
			Self::ServiceError(e) => match e {
				rmcp::ServiceError::McpError(_) => "mcp_error".to_string(),
				rmcp::ServiceError::Timeout { timeout: _ } => "timeout".to_string(),
				rmcp::ServiceError::Cancelled { reason } => {
					reason.clone().unwrap_or("cancelled".to_string())
				},
				rmcp::ServiceError::UnexpectedResponse => "unexpected_response".to_string(),
				rmcp::ServiceError::Transport(_) => "transport_error".to_string(),
				_ => "unknown".to_string(),
			},
			Self::OpenAPIError(_) => "openapi_error".to_string(),
		}
	}
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

// UpstreamTarget defines a source for MCP information.
#[derive(Debug)]
pub(crate) enum UpstreamTarget {
	Mcp(RunningService<RoleClient, crate::relay::pool::PeerClientHandler>),
	OpenAPI(openapi::Handler),
}

impl UpstreamTarget {
	pub(crate) async fn initialize(
		&self,
		request: InitializeRequestParam,
	) -> Result<InitializeResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => {
				let result = m
					.send_request(ClientRequest::InitializeRequest(InitializeRequest {
						method: Default::default(),
						params: request,
						extensions: rmcp::model::Extensions::new(),
					}))
					.await?;
				match result {
					ServerResult::InitializeResult(result) => Ok(result),
					_ => Err(UpstreamError::ServiceError(
						rmcp::ServiceError::UnexpectedResponse,
					)),
				}
			},
			UpstreamTarget::OpenAPI(_) => Ok(InitializeResult::default()),
		}
	}

	pub(crate) async fn list_tools(
		&self,
		request: Option<PaginatedRequestParam>,
		rq_ctx: &RqCtx,
	) -> Result<ListToolsResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => {
				let mut extensions = rmcp::model::Extensions::new();
				extensions.insert(rq_ctx.clone());
				let result = m
					.send_request(ClientRequest::ListToolsRequest(ListToolsRequest {
						method: Default::default(),
						params: request,
						extensions,
					}))
					.await?;
				match result {
					ServerResult::ListToolsResult(result) => Ok(result),
					_ => Err(UpstreamError::ServiceError(
						rmcp::ServiceError::UnexpectedResponse,
					)),
				}
			},
			UpstreamTarget::OpenAPI(m) => Ok(ListToolsResult {
				next_cursor: None,
				tools: m.tools(),
			}),
		}
	}

	pub(crate) async fn get_prompt(
		&self,
		request: GetPromptRequestParam,
		rq_ctx: &RqCtx,
	) -> Result<GetPromptResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => {
				let mut extensions = rmcp::model::Extensions::new();
				extensions.insert(rq_ctx.clone());
				let result = m
					.send_request(ClientRequest::GetPromptRequest(GetPromptRequest {
						method: Default::default(),
						params: request,
						extensions,
					}))
					.await?;
				match result {
					ServerResult::GetPromptResult(result) => Ok(result),
					_ => Err(UpstreamError::ServiceError(
						rmcp::ServiceError::UnexpectedResponse,
					)),
				}
			},
			UpstreamTarget::OpenAPI(_) => Ok(GetPromptResult {
				description: None,
				messages: vec![],
			}),
		}
	}

	pub(crate) async fn list_prompts(
		&self,
		request: Option<PaginatedRequestParam>,
		rq_ctx: &RqCtx,
	) -> Result<ListPromptsResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => {
				let mut extensions = rmcp::model::Extensions::new();
				extensions.insert(rq_ctx.clone());
				let result = m
					.send_request(ClientRequest::ListPromptsRequest(ListPromptsRequest {
						method: Default::default(),
						params: request,
						extensions,
					}))
					.await?;
				match result {
					ServerResult::ListPromptsResult(result) => Ok(result),
					_ => Err(UpstreamError::ServiceError(
						rmcp::ServiceError::UnexpectedResponse,
					)),
				}
			},
			UpstreamTarget::OpenAPI(_) => Ok(ListPromptsResult {
				next_cursor: None,
				prompts: vec![],
			}),
		}
	}

	pub(crate) async fn list_resources(
		&self,
		request: Option<PaginatedRequestParam>,
		rq_ctx: &RqCtx,
	) -> Result<ListResourcesResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => {
				let mut extensions = rmcp::model::Extensions::new();
				extensions.insert(rq_ctx.clone());
				let result = m
					.send_request(ClientRequest::ListResourcesRequest(ListResourcesRequest {
						method: Default::default(),
						params: request,
						extensions,
					}))
					.await?;
				match result {
					ServerResult::ListResourcesResult(result) => Ok(result),
					_ => Err(UpstreamError::ServiceError(
						rmcp::ServiceError::UnexpectedResponse,
					)),
				}
			},
			UpstreamTarget::OpenAPI(_) => Ok(ListResourcesResult {
				next_cursor: None,
				resources: vec![],
			}),
		}
	}

	pub(crate) async fn list_resource_templates(
		&self,
		request: Option<PaginatedRequestParam>,
		rq_ctx: &RqCtx,
	) -> Result<ListResourceTemplatesResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => {
				let mut extensions = rmcp::model::Extensions::new();
				extensions.insert(rq_ctx.clone());
				let result = m
					.send_request(ClientRequest::ListResourceTemplatesRequest(
						ListResourceTemplatesRequest {
							method: Default::default(),
							params: request,
							extensions,
						},
					))
					.await?;
				match result {
					ServerResult::ListResourceTemplatesResult(result) => Ok(result),
					_ => Err(UpstreamError::ServiceError(
						rmcp::ServiceError::UnexpectedResponse,
					)),
				}
			},
			UpstreamTarget::OpenAPI(_) => Ok(ListResourceTemplatesResult {
				next_cursor: None,
				resource_templates: vec![],
			}),
		}
	}

	pub(crate) async fn read_resource(
		&self,
		request: ReadResourceRequestParam,
		rq_ctx: &RqCtx,
	) -> Result<ReadResourceResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => {
				let mut extensions = rmcp::model::Extensions::new();
				extensions.insert(rq_ctx.clone());
				let result = m
					.send_request(ClientRequest::ReadResourceRequest(ReadResourceRequest {
						method: Default::default(),
						params: request,
						extensions,
					}))
					.await?;
				match result {
					ServerResult::ReadResourceResult(result) => Ok(result),
					_ => Err(UpstreamError::ServiceError(
						rmcp::ServiceError::UnexpectedResponse,
					)),
				}
			},
			UpstreamTarget::OpenAPI(_) => Ok(ReadResourceResult { contents: vec![] }),
		}
	}

	pub(crate) async fn call_tool(
		&self,
		request: CallToolRequestParam,
		rq_ctx: &RqCtx,
	) -> Result<CallToolResult, UpstreamError> {
		match self {
			UpstreamTarget::Mcp(m) => {
				let mut extensions = rmcp::model::Extensions::new();
				extensions.insert(rq_ctx.clone());
				let result = m
					.send_request(ClientRequest::CallToolRequest(CallToolRequest {
						method: Default::default(),
						params: request,
						extensions,
					}))
					.await?;
				match result {
					ServerResult::CallToolResult(result) => Ok(result),
					_ => Err(UpstreamError::ServiceError(
						rmcp::ServiceError::UnexpectedResponse,
					)),
				}
			},
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
