use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::Display;
use std::io::Cursor;
use std::marker::PhantomData;
use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::sync::Arc;
use std::{cmp, net};

use anyhow::anyhow;
use indexmap::IndexMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use openapiv3::OpenAPI as OpenAPIv3;
use openapiv3_1::OpenApi as OpenAPIv3_1;
use regex::Regex;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, ServerConfig};
use rustls_pemfile::Item;

use secrecy::SecretString;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::http::auth::BackendAuth;
use crate::http::jwt::Jwt;
use crate::http::localratelimit::RateLimit;
use crate::http::{
	HeaderName, HeaderValue, StatusCode, ext_authz, filters, remoteratelimit, retry, status, timeout,
	uri,
};
use crate::mcp::rbac::RuleSet;
use crate::transport::tls;
use crate::types::discovery::NamespacedHostname;
use crate::types::proto;
use crate::types::proto::ProtoError;
use crate::*;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bind {
	pub key: BindName,
	pub address: SocketAddr,
	pub listeners: ListenerSet,
}

pub type BindName = Strng;
pub type ListenerName = Strng;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listener {
	pub key: ListenerKey,
	// User facing name
	pub name: ListenerName,
	// User facing name
	pub gateway_name: GatewayName,
	/// Can be a wildcard
	pub hostname: Strng,
	pub protocol: ListenerProtocol,
	pub routes: RouteSet,
	pub tcp_routes: TCPRouteSet,
}

pub type GatewayName = Strng;

#[derive(Debug, Clone)]
pub struct TLSConfig {
	pub config: Arc<ServerConfig>,
}

impl serde::Serialize for TLSConfig {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// TODO: store raw pem
		serializer.serialize_none()
	}
}

pub fn parse_cert(mut cert: &[u8]) -> Result<Vec<CertificateDer<'static>>, anyhow::Error> {
	let mut reader = std::io::BufReader::new(Cursor::new(&mut cert));
	let parsed: Result<Vec<_>, _> = rustls_pemfile::read_all(&mut reader).collect();
	parsed?
		.into_iter()
		.map(|p| {
			let Item::X509Certificate(der) = p else {
				return Err(anyhow!("no certificate"));
			};
			Ok(der)
		})
		.collect::<Result<Vec<_>, _>>()
}

pub fn parse_key(mut key: &[u8]) -> Result<PrivateKeyDer<'static>, anyhow::Error> {
	let mut reader = std::io::BufReader::new(Cursor::new(&mut key));
	let parsed = rustls_pemfile::read_one(&mut reader)?;
	let parsed = parsed.ok_or_else(|| anyhow!("no key"))?;
	match parsed {
		Item::Pkcs8Key(c) => Ok(PrivateKeyDer::Pkcs8(c)),
		Item::Pkcs1Key(c) => Ok(PrivateKeyDer::Pkcs1(c)),
		_ => Err(anyhow!("unsupported key")),
	}
}
#[derive(Debug, Clone, serde::Serialize)]
pub enum ListenerProtocol {
	HTTP,
	HTTPS(TLSConfig),
	TLS(TLSConfig),
	TCP,
	HBONE,
}

impl ListenerProtocol {
	pub fn tls(&self) -> Option<Arc<rustls::ServerConfig>> {
		match self {
			ListenerProtocol::HTTPS(t) | ListenerProtocol::TLS(t) => Some(t.config.clone()),
			_ => None,
		}
	}
}

pub type ListenerKey = Strng;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Route {
	// Internal name
	pub key: RouteKey,
	// User facing name of the route
	pub route_name: RouteName,
	// User facing name of the rule
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub rule_name: Option<RouteRuleName>,
	/// Can be a wildcard
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub hostnames: Vec<Strng>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub matches: Vec<RouteMatch>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub filters: Vec<RouteFilter>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub backends: Vec<RouteBackend>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub policies: Option<TrafficPolicy>,
}

pub type RouteKey = Strng;
pub type RouteName = Strng;
pub type RouteRuleName = Strng;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct TCPRoute {
	// Internal name
	pub key: RouteKey,
	// User facing name of the route
	pub route_name: RouteName,
	// Can be a wildcard. Not applicable for TCP, only for TLS
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub hostnames: Vec<Strng>,
	// User facing name of the rule
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub rule_name: Option<RouteRuleName>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub backends: Vec<TCPRouteBackend>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct TCPRouteBackend {
	#[serde(default = "default_weight")]
	pub weight: usize,
	pub backend: SimpleBackend,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct RouteMatch {
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub headers: Vec<HeaderMatch>,
	pub path: PathMatch,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub method: Option<MethodMatch>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub query: Vec<QueryMatch>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MethodMatch {
	pub method: Strng,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct HeaderMatch {
	#[serde(serialize_with = "ser_display", deserialize_with = "de_parse")]
	#[cfg_attr(feature = "schema", schemars(with = "String"))]
	pub name: HeaderName,
	pub value: HeaderValueMatch,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct QueryMatch {
	#[serde(serialize_with = "ser_display")]
	pub name: Strng,
	pub value: QueryValueMatch,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum QueryValueMatch {
	Exact(Strng),
	Regex(
		#[serde(with = "serde_regex")]
		#[cfg_attr(feature = "schema", schemars(with = "String"))]
		regex::Regex,
	),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum HeaderValueMatch {
	Exact(
		#[serde(serialize_with = "ser_bytes", deserialize_with = "de_parse")]
		#[cfg_attr(feature = "schema", schemars(with = "String"))]
		HeaderValue,
	),
	Regex(
		#[serde(with = "serde_regex")]
		#[cfg_attr(feature = "schema", schemars(with = "String"))]
		regex::Regex,
	),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum PathMatch {
	Exact(Strng),
	PathPrefix(Strng),
	Regex(
		#[serde(with = "serde_regex")]
		#[cfg_attr(feature = "schema", schemars(with = "String"))]
		regex::Regex,
		usize,
	),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum RouteFilter {
	RequestHeaderModifier(filters::HeaderModifier),
	ResponseHeaderModifier(filters::HeaderModifier),
	RequestRedirect(filters::RequestRedirect),
	UrlRewrite(filters::UrlRewrite),
	RequestMirror(filters::RequestMirror),
	// TODO: xds support
	DirectResponse(filters::DirectResponse),
	#[serde(rename = "cors")]
	CORS(http::cors::Cors),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct TrafficPolicy {
	pub timeout: timeout::Policy,
	pub retry: Option<retry::Policy>,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum HostRedirect {
	Full(Strng),
	Host(Strng),
	Port(NonZeroU16),
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum PathRedirect {
	Full(Strng),
	Prefix(Strng),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct RouteBackend {
	#[serde(default = "default_weight")]
	pub weight: usize,
	#[serde(flatten)]
	pub backend: Backend,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub filters: Vec<RouteFilter>,
}

fn default_weight() -> usize {
	1
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum Backend {
	Service {
		name: NamespacedHostname,
		port: u16,
	},
	#[serde(rename = "host")]
	Opaque(Target), // Hostname or IP
	Dynamic {},
	#[serde(rename = "mcp")]
	MCP(McpBackend),
	#[serde(rename = "ai")]
	AI(crate::llm::AIBackend),
	Invalid,
}

impl From<SimpleBackend> for Backend {
	fn from(value: SimpleBackend) -> Self {
		match value {
			SimpleBackend::Service { name, port } => Backend::Service { name, port },
			SimpleBackend::Opaque(target) => Backend::Opaque(target),
			SimpleBackend::Invalid => Backend::Invalid,
		}
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum SimpleBackend {
	Service {
		name: NamespacedHostname,
		port: u16,
	},
	#[serde(rename = "host")]
	Opaque(Target), // Hostname or IP
	Invalid,
}

impl Backend {
	pub fn name(&self) -> Strng {
		match self {
			Backend::Service { name, port } => {
				strng::format!("service/{name}:{port}")
			},
			Backend::Opaque(v) => strng::format!("opaque/{v}"),
			Backend::MCP(mcp) => strng::format!("mcp/{}", mcp.name),
			Backend::AI(ai) => strng::format!("ai/{}", ai.name),
			// TODO: give it a name
			Backend::Dynamic {} => strng::format!("dynamic"),
			Backend::Invalid => strng::format!("invalid"),
		}
	}
}

pub type BackendName = Strng;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct McpBackend {
	pub name: BackendName,
	pub targets: Vec<Arc<McpTarget>>,
}

impl McpBackend {
	pub fn find(&self, name: &str) -> Option<Arc<McpTarget>> {
		self
			.targets
			.iter()
			.find(|target| target.name.as_str() == name)
			.cloned()
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct McpTarget {
	pub name: McpTargetName,
	#[serde(flatten)]
	pub spec: McpTargetSpec,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub filters: Vec<mcp::relay::upstream::Filter>,
}

type McpTargetName = Strng;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum McpTargetSpec {
	#[serde(rename = "sse")]
	Sse(SseTargetSpec),
	#[serde(rename = "mcp")]
	Mcp(StreamableHTTPTargetSpec),
	#[serde(rename = "stdio")]
	Stdio {
		cmd: String,
		#[serde(default, skip_serializing_if = "Vec::is_empty")]
		args: Vec<String>,
		#[serde(default, skip_serializing_if = "HashMap::is_empty")]
		env: HashMap<String, String>,
	},
	#[serde(rename = "openapi")]
	OpenAPI(OpenAPITarget),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SseTargetSpec {
	// TODO: reference a service
	pub host: String,
	pub port: u32,
	pub path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct StreamableHTTPTargetSpec {
	// TODO: reference a service
	pub host: String,
	pub port: u32,
	pub path: String,
}

/// Unified OpenAPI specification that can handle both 3.0 and 3.1 versions
#[derive(Clone)]
pub enum OpenAPI {
	V3_0(Arc<OpenAPIv3>),
	V3_1(Arc<OpenAPIv3_1>),
}

impl std::fmt::Debug for OpenAPI {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			OpenAPI::V3_0(spec) => f.debug_tuple("V3_0").field(&spec.openapi).finish(),
			OpenAPI::V3_1(_spec) => f.debug_tuple("V3_1").field(&"3.1").finish(),
		}
	}
}

impl OpenAPI {
	/// Get the OpenAPI version string
	pub fn version(&self) -> String {
		match self {
			OpenAPI::V3_0(spec) => spec.openapi.clone(),
			OpenAPI::V3_1(_spec) => "3.1".to_string(),
		}
	}
}

impl serde::Serialize for OpenAPI {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		// For serialization purposes, we'll serialize the version info
		// The actual schema content is not typically serialized
		match self {
			OpenAPI::V3_0(spec) => {
				let mut map = serializer.serialize_map(Some(2))?;
				map.serialize_entry("version", "3.0")?;
				map.serialize_entry("openapi", &spec.openapi)?;
				map.end()
			},
			OpenAPI::V3_1(_spec) => {
				let mut map = serializer.serialize_map(Some(2))?;
				map.serialize_entry("version", "3.1")?;
				map.serialize_entry("openapi", "3.1")?;
				map.end()
			},
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum OpenAPIVersion {
	V3_0,
	V3_1,
}

/// Detect OpenAPI version from the content string
pub fn detect_openapi_version(content: &str) -> Result<OpenAPIVersion, Box<dyn std::error::Error + Send + Sync>> {
	// Parse just enough to get the version field
	let value: serde_json::Value = yamlviajson::from_str(content)
		.map_err(|e| format!("Failed to parse content: {}", e))?;
	
	let version = value
		.get("openapi")
		.and_then(|v| v.as_str())
		.ok_or("Missing 'openapi' field")?;
	
	if version.starts_with("3.0") {
		Ok(OpenAPIVersion::V3_0)
	} else if version.starts_with("3.1") {
		Ok(OpenAPIVersion::V3_1)
	} else {
		Err(format!(
			"Unsupported OpenAPI version: {}. Only 3.0.x and 3.1.x are supported.",
			version
		).into())
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct OpenAPITarget {
	pub host: String,
	pub port: u32,
	#[serde(deserialize_with = "de_openapi")]
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	pub schema: Arc<OpenAPI>,
}

fn de_openapi<'a, D>(deserializer: D) -> Result<Arc<OpenAPI>, D::Error>
where
	D: serde::Deserializer<'a>,
{
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	#[serde(rename_all = "camelCase", deny_unknown_fields)]
	enum Serde {
		File(PathBuf),
		Inline(String),
		// Remote()
	}
	let s = Serde::deserialize(deserializer)?;

	let content = match s {
		Serde::File(f) => {
			let f = std::fs::read(f).map_err(serde::de::Error::custom)?;
			String::from_utf8(f).map_err(serde::de::Error::custom)?
		},
		Serde::Inline(s) => s,
	};

	// Detect the OpenAPI version and parse accordingly
	let version = detect_openapi_version(&content).map_err(serde::de::Error::custom)?;
	let schema = match version {
		OpenAPIVersion::V3_0 => {
			let spec: OpenAPIv3 = yamlviajson::from_str(&content)
				.map_err(serde::de::Error::custom)?;
			OpenAPI::V3_0(Arc::new(spec))
		},
		OpenAPIVersion::V3_1 => {
			let spec: OpenAPIv3_1 = yamlviajson::from_str(&content)
				.map_err(serde::de::Error::custom)?;
			OpenAPI::V3_1(Arc::new(spec))
		},
	};

	Ok(Arc::new(schema))
}

#[derive(Debug, Clone, Default)]
pub struct ListenerSet {
	pub inner: HashMap<ListenerKey, Arc<Listener>>,
}

impl serde::Serialize for ListenerSet {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.inner.serialize(serializer)
	}
}

impl ListenerSet {
	pub fn from_list<const N: usize>(l: [Listener; N]) -> ListenerSet {
		let mut listeners = HashMap::with_capacity(l.len());
		for ls in l.into_iter() {
			listeners.insert(ls.key.clone(), Arc::new(ls));
		}
		ListenerSet { inner: listeners }
	}

	pub fn best_match(&self, host: &str) -> Option<Arc<Listener>> {
		if let Some(best) = self.inner.values().find(|l| l.hostname == host) {
			trace!("found best match for {host} (exact)");
			return Some(best.clone());
		}
		if let Some(best) = self
			.inner
			.values()
			.sorted_by_key(|l| -(l.hostname.len() as i64))
			.find(|l| l.hostname.starts_with("*") && host.ends_with(&l.hostname.as_str()[1..]))
		{
			trace!("found best match for {host} (wildcard {})", best.hostname);
			return Some(best.clone());
		}
		trace!("trying to find best match for {host} (empty hostname)");
		self.inner.values().find(|l| l.hostname.is_empty()).cloned()
	}

	pub fn insert(&mut self, k: ListenerKey, v: Listener) {
		self.inner.insert(k, Arc::new(v));
	}

	pub fn contains(&self, key: &ListenerKey) -> bool {
		self.inner.contains_key(key)
	}

	pub fn get(&self, key: &ListenerKey) -> Option<&Listener> {
		self.inner.get(key).map(Arc::as_ref)
	}

	pub fn get_exactly_one(&self) -> anyhow::Result<Arc<Listener>> {
		if self.inner.len() != 1 {
			anyhow::bail!("expecting only one listener for TCP");
		}
		self
			.inner
			.iter()
			.next()
			.ok_or_else(|| anyhow::anyhow!("expecting one listener"))
			.map(|(k, v)| v.clone())
	}

	pub fn remove(&mut self, key: &ListenerKey) {
		self.inner.remove(key);
	}

	pub fn iter(&self) -> impl Iterator<Item = &Listener> {
		self.inner.values().map(Arc::as_ref)
	}
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize)]
pub enum HostnameMatch {
	Exact(Strng),
	// *.example.com -> Wildcard(example.com)
	Wildcard(Strng),
	None,
}

impl From<Strng> for HostnameMatch {
	fn from(s: Strng) -> Self {
		if let Some(s) = s.strip_prefix("*.") {
			HostnameMatch::Wildcard(strng::new(s))
		} else {
			HostnameMatch::Exact(s.clone())
		}
	}
}

impl HostnameMatch {
	pub fn all_matches_or_none(
		hostname: Option<&str>,
	) -> Box<dyn Iterator<Item = HostnameMatch> + '_> {
		match hostname {
			None => Box::new(std::iter::once(HostnameMatch::None)),
			Some(h) => Box::new(Self::all_matches(h)),
		}
	}
	pub fn all_matches(hostname: &str) -> impl Iterator<Item = HostnameMatch> + '_ {
		Self::all_actual_matches(hostname).chain(std::iter::once(HostnameMatch::None))
	}
	fn all_actual_matches(hostname: &str) -> impl Iterator<Item = HostnameMatch> + '_ {
		let start = if hostname.starts_with("*.") {
			None
		} else {
			Some(HostnameMatch::Exact(hostname.into()))
		};
		// Build wildcards in reverse order by collecting parts and building from longest to shortest
		let parts: Vec<_> = hostname.split('.').skip(1).collect();
		let wildcards = (0..parts.len()).map(move |i| {
			let suffix = parts[i..].join(".");
			HostnameMatch::Wildcard(suffix.into())
		});
		start.into_iter().chain(wildcards)
	}
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize)]
pub struct SingleRouteMatch {
	key: RouteKey,
	index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RouteSet {
	// Hostname -> []routes, sorted so that route matching can do a linear traversal
	inner: HashMap<HostnameMatch, Vec<SingleRouteMatch>>,
	// All routes
	all: HashMap<RouteKey, Route>,
}

impl serde::Serialize for RouteSet {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.all.serialize(serializer)
	}
}

impl RouteSet {
	pub fn from_list(l: Vec<Route>) -> RouteSet {
		let mut rs = RouteSet::default();
		for ls in l.into_iter() {
			rs.insert(ls);
		}
		rs
	}

	pub fn get_hostname(&self, hnm: &HostnameMatch) -> impl Iterator<Item = (&Route, &RouteMatch)> {
		self.inner.get(hnm).into_iter().flatten().flat_map(|rl| {
			self
				.all
				.get(&rl.key)
				.map(|r| (r, r.matches.get(rl.index).expect("corrupted state")))
		})
	}

	pub fn insert(&mut self, r: Route) {
		// Insert the route into all HashMap first so it's available during binary search
		self.all.insert(r.key.clone(), r.clone());

		for hostname_match in Self::hostname_matchers(&r) {
			let mut v = self.inner.entry(hostname_match).or_default();
			for (idx, m) in r.matches.iter().enumerate() {
				let to_insert = v.binary_search_by(|existing| {
					let have = self.all.get(&existing.key).expect("corrupted state");
					let have_match = have.matches.get(existing.index).expect("corrupted state");

					cmp::Ordering::reverse(Self::compare_route(m, have_match))
				});
				// TODO: replace old route
				let insert_idx = to_insert.unwrap_or_else(|pos| pos);
				v.insert(
					insert_idx,
					SingleRouteMatch {
						key: r.key.clone(),
						index: idx,
					},
				);
			}
		}
	}

	fn compare_route(a: &RouteMatch, b: &RouteMatch) -> Ordering {
		// Compare RouteMatch according to Gateway API sorting requirements
		// 1. Path match type (Exact > PathPrefix > Regex)
		let path_rank1 = get_path_rank(&a.path);
		let path_rank2 = get_path_rank(&b.path);
		if path_rank1 != path_rank2 {
			return cmp::Ordering::reverse(path_rank1.cmp(&path_rank2));
		}
		// 2. Path length (longer paths first)
		let path_len1 = get_path_length(&a.path);
		let path_len2 = get_path_length(&b.path);
		if path_len1 != path_len2 {
			return cmp::Ordering::reverse(path_len1.cmp(&path_len2)); // Reverse order for longer first
		}
		// 3. Method match (routes with method matches first)
		let method1 = a.method.is_some();
		let method2 = b.method.is_some();
		if method1 != method2 {
			return cmp::Ordering::reverse(method1.cmp(&method2));
		}
		// 4. Number of header matches (more headers first)
		let header_count1 = a.headers.len();
		let header_count2 = b.headers.len();
		if header_count1 != header_count2 {
			return cmp::Ordering::reverse(header_count1.cmp(&header_count2));
		}
		// 5. Number of query matches (more query params first)
		let query_count1 = a.query.len();
		let query_count2 = b.query.len();
		cmp::Ordering::reverse(query_count1.cmp(&query_count2))
	}

	pub fn contains(&self, key: &RouteKey) -> bool {
		self.all.contains_key(key)
	}

	pub fn remove(&mut self, key: &RouteKey) {
		let Some(old_route) = self.all.remove(key) else {
			return;
		};

		for hostname_match in Self::hostname_matchers(&old_route) {
			let mut entry = self
				.inner
				.entry(hostname_match)
				.and_modify(|v| v.retain(|r| &r.key != key));
			match entry {
				Entry::Occupied(v) => {
					if v.get().is_empty() {
						v.remove();
					}
				},
				Entry::Vacant(_) => {},
			}
		}
	}

	fn hostname_matchers(r: &Route) -> Vec<HostnameMatch> {
		if r.hostnames.is_empty() {
			vec![HostnameMatch::None]
		} else {
			r.hostnames
				.iter()
				.map(|h| HostnameMatch::from(h.clone()))
				.collect()
		}
	}

	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

#[derive(Debug, Clone, Default)]
pub struct TCPRouteSet {
	// Hostname -> []routes, sorted so that route matching can do a linear traversal
	inner: HashMap<HostnameMatch, Vec<RouteKey>>,
	// All routes
	all: HashMap<RouteKey, TCPRoute>,
}

impl serde::Serialize for TCPRouteSet {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.all.serialize(serializer)
	}
}

impl TCPRouteSet {
	pub fn from_list(l: Vec<TCPRoute>) -> Self {
		let mut rs = Self::default();
		for ls in l.into_iter() {
			rs.insert(ls);
		}
		rs
	}

	pub fn get_hostname(&self, hnm: &HostnameMatch) -> Option<&TCPRoute> {
		self
			.inner
			.get(hnm)
			.and_then(|r| r.first())
			.and_then(|rl| self.all.get(rl))
	}

	pub fn insert(&mut self, r: TCPRoute) {
		// Insert the route into all HashMap first so it's available during binary search
		self.all.insert(r.key.clone(), r.clone());

		for hostname_match in Self::hostname_matchers(&r) {
			let mut v = self.inner.entry(hostname_match).or_default();
			let to_insert = v.binary_search_by(|existing| {
				let have = self.all.get(existing).expect("corrupted state");
				// TODO: not sure that is right
				Ordering::reverse(r.key.cmp(existing))
			});
			// TODO: replace old route
			let insert_idx = to_insert.unwrap_or_else(|pos| pos);
			v.insert(insert_idx, r.key.clone());
		}
	}

	fn compare_route(a: &RouteMatch, b: &RouteMatch) -> Ordering {
		// Compare RouteMatch according to Gateway API sorting requirements
		// 1. Path match type (Exact > PathPrefix > Regex)
		let path_rank1 = get_path_rank(&a.path);
		let path_rank2 = get_path_rank(&b.path);
		if path_rank1 != path_rank2 {
			return cmp::Ordering::reverse(path_rank1.cmp(&path_rank2));
		}
		// 2. Path length (longer paths first)
		let path_len1 = get_path_length(&a.path);
		let path_len2 = get_path_length(&b.path);
		if path_len1 != path_len2 {
			return cmp::Ordering::reverse(path_len1.cmp(&path_len2)); // Reverse order for longer first
		}
		// 3. Method match (routes with method matches first)
		let method1 = a.method.is_some();
		let method2 = b.method.is_some();
		if method1 != method2 {
			return cmp::Ordering::reverse(method1.cmp(&method2));
		}
		// 4. Number of header matches (more headers first)
		let header_count1 = a.headers.len();
		let header_count2 = b.headers.len();
		if header_count1 != header_count2 {
			return cmp::Ordering::reverse(header_count1.cmp(&header_count2));
		}
		// 5. Number of query matches (more query params first)
		let query_count1 = a.query.len();
		let query_count2 = b.query.len();
		cmp::Ordering::reverse(query_count1.cmp(&query_count2))
	}

	pub fn contains(&self, key: &RouteKey) -> bool {
		self.all.contains_key(key)
	}

	pub fn remove(&mut self, key: &RouteKey) {
		let Some(old_route) = self.all.remove(key) else {
			return;
		};

		for hostname_match in Self::hostname_matchers(&old_route) {
			let mut entry = self
				.inner
				.entry(hostname_match)
				.and_modify(|v| v.retain(|r| r != key));
			match entry {
				Entry::Occupied(v) => {
					if v.get().is_empty() {
						v.remove();
					}
				},
				Entry::Vacant(_) => {},
			}
		}
	}

	fn hostname_matchers(r: &TCPRoute) -> Vec<HostnameMatch> {
		if r.hostnames.is_empty() {
			vec![HostnameMatch::None]
		} else {
			r.hostnames
				.iter()
				.map(|h| HostnameMatch::from(h.clone()))
				.collect()
		}
	}

	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

// Helper functions for RouteMatch comparison
fn get_path_rank(path: &PathMatch) -> i32 {
	match path {
		// Best match: exact
		PathMatch::Exact(_) => 3,
		// Prefix/Regex -- we will defer to the length
		PathMatch::PathPrefix(_) => 2,
		PathMatch::Regex(_, _) => 2,
	}
}

fn get_path_length(path: &PathMatch) -> usize {
	match path {
		PathMatch::Exact(s) => s.len(),
		PathMatch::PathPrefix(s) => s.len(),
		PathMatch::Regex(_, l) => *l,
	}
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, serde::Serialize)]
pub enum IpFamily {
	Dual,
	IPv4,
	IPv6,
}

pub type PolicyName = Strng;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct TargetedPolicy {
	pub name: PolicyName,
	pub target: PolicyTarget,
	pub policy: Policy,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum PolicyTarget {
	Gateway(GatewayName),
	Listener(ListenerKey),
	Route(RouteName),
	RouteRule(RouteKey),
	RouteBackend(BackendName),
	Service(NamespacedHostname),
	McpTarget(McpTargetName),
	Opaque(Target),
	// not yet implemented.
	TODO,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Policy {
	// Supported targets: RouteBackend, only when Backend type is MCP
	McpAuthorization(McpAuthorization),
	McpAuthentication(McpAuthentication),

	// Support targets: Backend; single policy allowed
	A2a(A2aPolicy),
	// Supported targets: Backend; single policy allowed
	#[serde(rename = "backendTLS")]
	BackendTLS(http::backendtls::BackendTLS),
	// Supported targets: Backend; single policy allowed
	BackendAuth(BackendAuth),

	// Supported targets: Gateway < Route < RouteRule; single policy allowed
	// Transformation(),
	// Supported targets: Gateway < Route < RouteRule; single policy allowed
	#[serde(rename = "ai")]
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	AI(llm::Policy),
	// Supported targets: Gateway < Route < RouteRule; single policy allowed
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	LocalRateLimit(Vec<crate::http::localratelimit::RateLimit>),
	// Supported targets: Gateway < Route < RouteRule; single policy allowed
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	ExtAuthz(ext_authz::ExtAuthz),
	// Supported targets: Gateway < Route < RouteRule; single policy allowed
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	RemoteRateLimit(remoteratelimit::RemoteRateLimit),
	// Supported targets: Gateway < Route < RouteRule; single policy allowed
	// ExtProc(),
	// Supported targets: Gateway < Route < RouteRule; single policy allowed
	#[cfg_attr(feature = "schema", schemars(with = "serde_json::value::RawValue"))]
	JwtAuth(crate::http::jwt::Jwt),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct A2aPolicy {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct McpAuthorization(RuleSet);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct McpAuthentication {
	pub issuer: String,
	pub scopes: Vec<String>,
	pub audience: String,
	pub provider: Option<McpIDP>,
}

impl McpAuthentication {
	pub fn as_jwt(&self) -> anyhow::Result<http::jwt::LocalJwtConfig> {
		Ok(http::jwt::LocalJwtConfig {
			issuer: self.issuer.clone(),
			audiences: vec![self.audience.clone()],
			jwks: FileInlineOrRemote::Remote {
				url: match &self.provider {
					None | Some(McpIDP::Auth0 { .. }) => {
						format!("{}/.well-known/jwks.json", self.issuer).parse()?
					},
					Some(McpIDP::Keycloak { .. }) => {
						format!("{}/protocol/openid-connect/certs", self.issuer).parse()?
					},
					// Some(McpIDP::Keycloak { realm }) => format!("{}/realms/{realm}/protocol/openid-connect/certs", self.issuer).parse()?,
				},
			},
		})
	}
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum McpIDP {
	Auth0 {},
	Keycloak {},
}

impl McpAuthorization {
	pub fn into_inner(self) -> RuleSet {
		self.0
	}
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum Target {
	Address(SocketAddr),
	Hostname(Strng, u16),
}

impl<'de> serde::Deserialize<'de> for Target {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		serdes::de_parse(deserializer)
	}
}

impl serde::Serialize for Target {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

impl TryFrom<(&str, u16)> for Target {
	type Error = anyhow::Error;

	fn try_from((host, port): (&str, u16)) -> Result<Self, Self::Error> {
		match host.parse::<IpAddr>() {
			Ok(target) => Ok(Target::Address(SocketAddr::new(target, port))),
			Err(_) => Ok(Target::Hostname(host.into(), port)),
		}
	}
}

impl TryFrom<&str> for Target {
	type Error = anyhow::Error;

	fn try_from(hostport: &str) -> Result<Self, Self::Error> {
		let Some((host, port)) = hostport.split_once(":") else {
			anyhow::bail!("invalid host:port: {}", hostport);
		};
		let port: u16 = port.parse()?;
		(host, port).try_into()
	}
}

impl Display for Target {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let str = match self {
			Target::Address(addr) => addr.to_string(),
			Target::Hostname(hostname, port) => format!("{hostname}:{port}"),
		};
		write!(f, "{str}")
	}
}
