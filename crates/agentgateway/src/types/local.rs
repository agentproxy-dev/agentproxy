use crate::http::auth::BackendAuth;
use crate::http::backendtls::{BackendTLS, LocalBackendTLS};
use crate::http::jwt::{JwkError, Jwt};
use crate::http::{filters, retry, timeout};
use crate::llm::AIProvider;
use crate::store::LocalWorkload;
use crate::transport::tls;
use crate::types::agent::PolicyTarget::RouteRule;
use crate::types::agent::{
	A2aPolicy, Backend, BackendName, BackendReference, Bind, BindName, GatewayName, Listener,
	ListenerKey, ListenerProtocol, ListenerSet, McpAuthentication, McpAuthorization, McpBackend,
	PathMatch, Policy, PolicyTarget, Route, RouteBackend, RouteBackendReference, RouteFilter,
	RouteMatch, RouteName, RouteRuleName, RouteSet, SimpleBackend, SimpleBackendReference, TCPRoute,
	TCPRouteBackendReference, TCPRouteSet, TLSConfig, Target, TargetedPolicy, TrafficPolicy,
	parse_cert, parse_key,
};
use crate::types::discovery::{NamespacedHostname, Service};
use crate::*;
use agent_core::prelude::Strng;
use anyhow::{Error, anyhow, bail};
use itertools::Itertools;
use jsonwebtoken::jwk::{AlgorithmParameters, JwkSet, KeyAlgorithm};
use jsonwebtoken::{DecodingKey, Validation};
use rmcp::handler::server::router::tool::CallToolHandlerExt;
use rustls::{ClientConfig, ServerConfig};
use serde::de::DeserializeOwned;
use serde_with::serde_as;
use std::collections::HashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

impl NormalizedLocalConfig {
	pub async fn from(client: client::Client, s: &str) -> anyhow::Result<NormalizedLocalConfig> {
		// Avoid shell expanding the comment for schema. Probably there are better ways to do this!
		let s = s.replace("# yaml-language-server: $schema", "#");
		let s = shellexpand::full(&s)?;
		let config: LocalConfig = serdes::yamlviajson::from_str(&s)?;
		let t = convert(client, config).await?;
		Ok(t)
	}
}

#[derive(Debug, Clone)]
pub struct NormalizedLocalConfig {
	pub binds: Vec<Bind>,
	pub policies: Vec<TargetedPolicy>,
	pub backends: Vec<Backend>,
	// Note: here we use LocalWorkload since it conveys useful info, we could maybe change but not a problem
	// for now
	pub workloads: Vec<LocalWorkload>,
	pub services: Vec<Service>,
}

#[cfg(feature = "schema")]
pub fn generate_schema() -> String {
	let settings = schemars::generate::SchemaSettings::default().with(|s| s.inline_subschemas = true);
	let gens = schemars::SchemaGenerator::new(settings);
	let schema = gens.into_root_schema_for::<LocalConfig>();
	serde_json::to_string_pretty(&schema).unwrap()
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct LocalConfig {
	#[serde(default)]
	binds: Vec<LocalBind>,
	#[serde(default)]
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	workloads: Vec<LocalWorkload>,
	#[serde(default)]
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	services: Vec<Service>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct LocalBind {
	port: u16,
	listeners: Vec<LocalListener>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct LocalListener {
	// User facing name
	name: Option<Strng>,
	// User facing name of the Gateway. Option, one will be set if not.
	gateway_name: Option<Strng>,
	/// Can be a wildcard
	hostname: Option<Strng>,
	#[serde(default)]
	protocol: LocalListenerProtocol,
	tls: Option<LocalTLSServerConfig>,
	routes: Option<Vec<LocalRoute>>,
	tcp_routes: Option<Vec<LocalTCPRoute>>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE", deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
enum LocalListenerProtocol {
	#[default]
	HTTP,
	HTTPS,
	TLS,
	TCP,
	HBONE,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct LocalTLSServerConfig {
	cert: PathBuf,
	key: PathBuf,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct LocalRoute {
	#[serde(default, skip_serializing_if = "Option::is_none", rename = "name")]
	// User facing name of the route
	route_name: Option<RouteName>,
	// User facing name of the rule
	#[serde(default, skip_serializing_if = "Option::is_none")]
	rule_name: Option<RouteRuleName>,
	/// Can be a wildcard
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	hostnames: Vec<Strng>,
	#[serde(default = "default_matches")]
	matches: Vec<RouteMatch>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	policies: Option<FilterOrPolicy>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	backends: Vec<LocalRouteBackend>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct LocalRouteBackend {
	#[serde(default = "default_weight")]
	pub weight: usize,
	#[serde(flatten)]
	pub backend: LocalBackend,
	// TODO: add back per-backend filters
	// #[serde(default, skip_serializing_if = "Vec::is_empty")]
	// pub filters: Vec<RouteFilter>,
}

fn default_weight() -> usize {
	1
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum LocalBackend {
	// This one is a reference
	Service {
		name: NamespacedHostname,
		port: u16,
	},
	// Rest are inlined
	#[serde(rename = "host")]
	Opaque(Target), // Hostname or IP
	Dynamic {},
	#[serde(rename = "mcp")]
	MCP(McpBackend),
	#[serde(rename = "ai")]
	AI(crate::llm::AIBackend),
	Invalid,
}

impl LocalBackend {
	pub fn as_backend(&self, name: BackendName) -> Option<Backend> {
		match self {
			LocalBackend::Service { .. } => None, // These stay as references
			LocalBackend::Opaque(tgt) => Some(Backend::Opaque(name, tgt.clone())),
			LocalBackend::Dynamic { .. } => Some(Backend::Dynamic {}),
			LocalBackend::MCP(tgt) => Some(Backend::MCP(name, tgt.clone())),
			LocalBackend::AI(tgt) => Some(Backend::AI(name, tgt.clone())),
			LocalBackend::Invalid => Some(Backend::Invalid),
		}
	}
}

fn default_matches() -> Vec<RouteMatch> {
	vec![RouteMatch {
		headers: vec![],
		path: PathMatch::PathPrefix("/".into()),
		method: None,
		query: vec![],
	}]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct LocalTCPRoute {
	#[serde(default, skip_serializing_if = "Option::is_none", rename = "name")]
	// User facing name of the route
	route_name: Option<RouteName>,
	// User facing name of the rule
	#[serde(default, skip_serializing_if = "Option::is_none")]
	rule_name: Option<RouteRuleName>,
	/// Can be a wildcard
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	hostnames: Vec<Strng>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	policies: Option<TCPFilterOrPolicy>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	backends: Vec<LocalTCPRouteBackend>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct LocalTCPRouteBackend {
	#[serde(default = "default_weight")]
	pub weight: usize,
	pub backend: SimpleLocalBackend,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum SimpleLocalBackend {
	Service {
		name: NamespacedHostname,
		port: u16,
	},
	#[serde(rename = "host")]
	Opaque(Target), // Hostname or IP
	Invalid,
}

impl SimpleLocalBackend {
	pub fn as_backend(&self, name: BackendName) -> Option<Backend> {
		match self {
			SimpleLocalBackend::Service { .. } => None, // These stay as references
			SimpleLocalBackend::Opaque(tgt) => Some(Backend::Opaque(name, tgt.clone())),
			SimpleLocalBackend::Invalid => Some(Backend::Invalid),
		}
	}
}
#[serde_as]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct FilterOrPolicy {
	// Filters. Keep in sync with RouteFilter
	#[serde(default, skip_serializing_if = "Option::is_none")]
	request_header_modifier: Option<filters::HeaderModifier>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	response_header_modifier: Option<filters::HeaderModifier>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	request_redirect: Option<filters::RequestRedirect>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	url_rewrite: Option<filters::UrlRewrite>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	request_mirror: Option<LocalRequestMirror>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	direct_response: Option<filters::DirectResponse>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	cors: Option<http::cors::Cors>,

	// Policy
	#[serde(default, skip_serializing_if = "Option::is_none")]
	mcp_authorization: Option<McpAuthorization>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	mcp_authentication: Option<McpAuthentication>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	a2a: Option<A2aPolicy>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	ai: Option<llm::Policy>,
	#[serde(
		rename = "backendTLS",
		default,
		skip_serializing_if = "Option::is_none"
	)]
	backend_tls: Option<http::backendtls::LocalBackendTLS>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	backend_auth: Option<BackendAuth>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	local_rate_limit: Vec<crate::http::localratelimit::RateLimit>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	remote_rate_limit: Option<crate::http::remoteratelimit::RemoteRateLimit>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	jwt_auth: Option<crate::http::jwt::LocalJwtConfig>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	ext_authz: Option<crate::http::ext_authz::ExtAuthz>,

	// TrafficPolicy
	#[serde(default, skip_serializing_if = "Option::is_none")]
	timeout: Option<timeout::Policy>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	retry: Option<retry::Policy>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct TCPFilterOrPolicy {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	backend_tls: Option<LocalBackendTLS>,
}

async fn convert(client: client::Client, i: LocalConfig) -> anyhow::Result<NormalizedLocalConfig> {
	let LocalConfig {
		binds,
		workloads,
		services,
	} = i;
	let mut all_policies = vec![];
	let mut all_backends = vec![];
	let mut all_binds = vec![];
	for b in binds {
		let bind_name = strng::format!("bind/{}", b.port);
		let mut ls = ListenerSet::default();
		for (idx, l) in b.listeners.into_iter().enumerate() {
			let (l, pol, backends) = convert_listener(client.clone(), bind_name.clone(), idx, l).await?;
			all_policies.extend_from_slice(&pol);
			all_backends.extend_from_slice(&backends);
			ls.insert(l.key.clone(), l)
		}
		let b = Bind {
			key: bind_name,
			address: SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), b.port),
			listeners: ls,
		};
		all_binds.push(b)
	}
	Ok(NormalizedLocalConfig {
		binds: all_binds,
		policies: all_policies,
		backends: all_backends,
		workloads,
		services,
	})
}

async fn convert_listener(
	client: client::Client,
	bind_name: BindName,
	idx: usize,
	l: LocalListener,
) -> anyhow::Result<(Listener, Vec<TargetedPolicy>, Vec<Backend>)> {
	let LocalListener {
		name,
		gateway_name,
		hostname,
		protocol,
		tls,
		routes,
		tcp_routes,
	} = l;

	let protocol = match protocol {
		LocalListenerProtocol::HTTP => {
			if routes.is_none() {
				bail!("protocol HTTP requires 'routes'")
			}
			ListenerProtocol::HTTP
		},
		LocalListenerProtocol::HTTPS => {
			if routes.is_none() {
				bail!("protocol HTTPS requires 'routes'")
			}
			ListenerProtocol::HTTPS(convert_tls_server(
				tls.ok_or(anyhow!("HTTPS listener requires 'tls'"))?,
			)?)
		},
		LocalListenerProtocol::TLS => {
			if tcp_routes.is_none() {
				bail!("protocol TLS requires 'tcpRoutes'")
			}
			ListenerProtocol::TLS(convert_tls_server(
				tls.ok_or(anyhow!("TLS listener requires 'tls'"))?,
			)?)
		},
		LocalListenerProtocol::TCP => {
			if tcp_routes.is_none() {
				bail!("protocol TCP requires 'tcpRoutes'")
			}
			ListenerProtocol::TCP
		},
		LocalListenerProtocol::HBONE => ListenerProtocol::HBONE,
	};
	if tcp_routes.is_some() && routes.is_some() {
		bail!("only 'routes' or 'tcpRoutes' may be set");
	}
	let name = name.unwrap_or_else(|| strng::format!("listener{}", idx));
	let gateway_name: GatewayName = gateway_name.unwrap_or(bind_name);
	let key: ListenerKey = strng::format!("{}/{}", name, gateway_name);

	let mut all_policies = vec![];
	let mut all_backends = vec![];

	let mut rs = RouteSet::default();
	for (idx, l) in routes.into_iter().flatten().enumerate() {
		let (route, policies, backends) = convert_route(client.clone(), l, idx, key.clone()).await?;
		all_policies.extend_from_slice(&policies);
		all_backends.extend_from_slice(&backends);
		rs.insert(route)
	}

	let mut trs = TCPRouteSet::default();
	for (idx, l) in tcp_routes.into_iter().flatten().enumerate() {
		let (route, policies) = convert_tcp_route(l, idx, key.clone()).await?;
		all_policies.extend_from_slice(&policies);
		trs.insert(route)
	}

	let l = Listener {
		key,
		name,
		gateway_name,
		hostname: hostname.unwrap_or_default(),
		protocol,
		routes: rs,
		tcp_routes: trs,
	};
	Ok((l, all_policies, all_backends))
}

async fn convert_route(
	client: client::Client,
	lr: LocalRoute,
	idx: usize,
	listener_key: ListenerKey,
) -> anyhow::Result<(Route, Vec<TargetedPolicy>, Vec<Backend>)> {
	let LocalRoute {
		route_name,
		rule_name,
		hostnames,
		matches,
		policies,
		backends,
	} = lr;

	let route_name = route_name.unwrap_or_else(|| strng::format!("route{}", idx));
	let key = strng::format!(
		"{}/{}/{}",
		listener_key,
		route_name,
		rule_name.clone().unwrap_or_else(|| strng::new("default"))
	);
	let mut filters = vec![];
	let mut traffic_policy: Option<TrafficPolicy> = None;
	let mut external_policies = vec![];
	let mut pol = 0;
	let mut tgt = |p: Policy| {
		pol += 1;
		TargetedPolicy {
			name: format!("{key}/{pol}").into(),
			target: RouteRule(key.clone()),
			policy: p,
		}
	};

	let (refs, mut external_backends): (Vec<_>, Vec<Option<Backend>>) = backends
		.into_iter()
		.map(|b| {
			let bref = match &b.backend {
				LocalBackend::Service { name, port } => BackendReference::Service {
					name: name.clone(),
					port: *port,
				},
				LocalBackend::Invalid => BackendReference::Invalid,
				_ => BackendReference::Backend(key.clone()),
			};
			let backend = b.backend.as_backend(bref.name());
			let bref = RouteBackendReference {
				weight: b.weight,
				backend: bref,
				filters: vec![],
				// filters: b.filters,
			};
			(bref, backend)
		})
		.unzip();
	let mut be_pol = 0;
	let mut backend_tgt = |p: Policy| {
		if refs.len() != 1 {
			anyhow::bail!("backend policies currently only work with exactly 1 backend")
		}
		let be = refs.first().unwrap();
		be_pol += 1;
		Ok(TargetedPolicy {
			name: format!("{key}/backend-{be_pol}").into(),
			target: PolicyTarget::Backend(be.backend.name()),
			policy: p,
		})
	};

	let mut traffic_policy = TrafficPolicy {
		timeout: timeout::Policy::default(),
		retry: None,
	};
	if let Some(pol) = policies {
		let FilterOrPolicy {
			request_header_modifier,
			response_header_modifier,
			request_redirect,
			url_rewrite,
			request_mirror,
			direct_response,
			cors,
			mcp_authorization,
			mcp_authentication,
			a2a,
			ai,
			backend_tls,
			backend_auth,
			local_rate_limit,
			remote_rate_limit,
			jwt_auth,
			ext_authz,
			timeout,
			retry,
		} = pol;
		if let Some(p) = request_header_modifier {
			filters.push(RouteFilter::RequestHeaderModifier(p));
		}
		if let Some(p) = response_header_modifier {
			filters.push(RouteFilter::ResponseHeaderModifier(p));
		}
		if let Some(p) = request_redirect {
			filters.push(RouteFilter::RequestRedirect(p));
		}
		if let Some(p) = url_rewrite {
			filters.push(RouteFilter::UrlRewrite(p));
		}
		if let Some(p) = request_mirror {
			let (bref, backend) = to_simple_backend_and_ref(strng::format!("{}/mirror", key), &p.backend);
			let pol = filters::RequestMirror {
				backend: bref,
				percentage: p.percentage,
			};
			external_backends.push(backend);
			filters.push(RouteFilter::RequestMirror(pol));
		}
		if let Some(p) = direct_response {
			filters.push(RouteFilter::DirectResponse(p));
		}
		if let Some(p) = cors {
			filters.push(RouteFilter::CORS(p));
		}

		if let Some(p) = mcp_authorization {
			external_policies.push(backend_tgt(Policy::McpAuthorization(p))?)
		}
		if let Some(p) = mcp_authentication {
			let jp = p.as_jwt()?;
			external_policies.push(backend_tgt(Policy::McpAuthentication(p))?);
			external_policies.push(tgt(Policy::JwtAuth(jp.try_into(client.clone()).await?)));
		}
		if let Some(p) = a2a {
			external_policies.push(backend_tgt(Policy::A2a(p))?)
		}
		if let Some(p) = ai {
			external_policies.push(backend_tgt(Policy::AI(p))?)
		}
		if let Some(p) = backend_tls {
			external_policies.push(backend_tgt(Policy::BackendTLS(p.try_into()?))?)
		}
		if let Some(p) = backend_auth {
			external_policies.push(backend_tgt(Policy::BackendAuth(p))?)
		}
		if let Some(p) = jwt_auth {
			external_policies.push(tgt(Policy::JwtAuth(p.try_into(client.clone()).await?)))
		}
		if let Some(p) = ext_authz {
			external_policies.push(tgt(Policy::ExtAuthz(p)))
		}
		if !local_rate_limit.is_empty() {
			external_policies.push(tgt(Policy::LocalRateLimit(local_rate_limit)))
		}
		if let Some(p) = remote_rate_limit {
			external_policies.push(tgt(Policy::RemoteRateLimit(p)))
		}

		if let Some(p) = timeout {
			traffic_policy.timeout = p;
		}
		if let Some(p) = retry {
			traffic_policy.retry = Some(p);
		}
	}
	let route = Route {
		key,
		route_name,
		rule_name,
		hostnames,
		matches,
		filters,
		backends: refs,
		policies: Some(traffic_policy),
	};
	Ok((
		route,
		external_policies,
		external_backends.into_iter().flatten().collect(),
	))
}

async fn convert_tcp_route(
	lr: LocalTCPRoute,
	idx: usize,
	listener_key: ListenerKey,
) -> anyhow::Result<(TCPRoute, Vec<TargetedPolicy>)> {
	let LocalTCPRoute {
		route_name,
		rule_name,
		hostnames,
		policies,
		backends,
	} = lr;

	let route_name = route_name.unwrap_or_else(|| strng::format!("tcproute{}", idx));
	let key = strng::format!(
		"{}/{}/{}",
		listener_key,
		route_name,
		rule_name.clone().unwrap_or_else(|| strng::new("default"))
	);
	let mut traffic_policy: Option<TrafficPolicy> = None;
	let mut external_policies = vec![];
	let mut pol = 0;
	let mut tgt = |p: Policy| {
		pol += 1;
		TargetedPolicy {
			name: format!("{key}/{pol}").into(),
			target: RouteRule(key.clone()),
			policy: p,
		}
	};
	let mut be_pol = 0;
	let mut backend_tgt = |p: Policy| {
		if backends.len() != 1 {
			anyhow::bail!("backend policies currently only work with exactly 1 backend")
		}

		let (refs, to_add): (Vec<_>, Vec<Option<Backend>>) = backends
			.into_iter()
			.map(|b| {
				let (bref, backend) = to_simple_backend_and_ref(key.clone(), &b.backend);
				let bref = TCPRouteBackendReference {
					weight: b.weight,
					backend: bref,
				};
				(bref, backend)
			})
			.unzip();
		let be = refs.first().unwrap();
		be_pol += 1;
		Ok(TargetedPolicy {
			name: format!("{key}/backend-{be_pol}").into(),
			target: PolicyTarget::Backend(be.backend.name()),
			policy: p,
		})
	};

	let mut traffic_policy = TrafficPolicy {
		timeout: timeout::Policy::default(),
		retry: None,
	};
	if let Some(pol) = policies {
		let TCPFilterOrPolicy { backend_tls } = pol;
		if let Some(p) = backend_tls {
			external_policies.push(backend_tgt(Policy::BackendTLS(p.try_into()?))?)
		}
	}
	let route = TCPRoute {
		key,
		route_name,
		rule_name,
		hostnames,
		backends: todo!(),
	};
	Ok((route, external_policies))
}

fn to_simple_backend_and_ref(
	name: BackendName,
	b: &SimpleLocalBackend,
) -> (SimpleBackendReference, Option<Backend>) {
	let bref = match &b {
		SimpleLocalBackend::Service { name, port } => SimpleBackendReference::Service {
			name: name.clone(),
			port: *port,
		},
		SimpleLocalBackend::Invalid => SimpleBackendReference::Invalid,
		_ => SimpleBackendReference::Backend(name.clone()),
	};
	let backend = b.as_backend(name);
	(bref, backend)
}

fn convert_tls_server(tls: LocalTLSServerConfig) -> anyhow::Result<TLSConfig> {
	let cert = fs_err::read(tls.cert)?;
	let cert_chain = crate::types::agent::parse_cert(&cert)?;
	let key = fs_err::read(tls.key)?;
	let private_key = crate::types::agent::parse_key(&key)?;

	let mut ccb = ServerConfig::builder_with_provider(transport::tls::provider())
		.with_protocol_versions(transport::tls::ALL_TLS_VERSIONS)
		.expect("server config must be valid")
		.with_no_client_auth()
		.with_single_cert(cert_chain, private_key)?;
	ccb.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
	Ok(TLSConfig {
		config: Arc::new(ccb),
	})
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct LocalRequestMirror {
	pub backend: SimpleLocalBackend,
	// 0.0-1.0
	pub percentage: f64,
}
