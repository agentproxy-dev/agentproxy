use std::error::Error as StdErr;
use std::fmt;
use std::fmt::Formatter;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::Level;
use tracing::{error, instrument, warn};

pub use client::*;
pub use metrics::*;
pub use types::*;

use self::envoy::service::discovery::v3::DeltaDiscoveryRequest;
use crate::proto::agentgateway::dev::a2a::target::Target as A2aXdsTarget;
use crate::proto::agentgateway::dev::common::BackendAuth as XdsAuth;
use crate::proto::agentgateway::dev::common::backend_auth::Auth as XdsAuthSpec;
use crate::proto::agentgateway::dev::listener::Listener as XdsListener;
use crate::proto::agentgateway::dev::mcp::target::Target as McpXdsTarget;
use crate::proto::agentgateway::dev::rbac::RuleSet as XdsRbac;
use crate::rbac;
use crate::strng::Strng;
use std::collections::HashMap;

use crate::inbound;
use crate::outbound;
use serde::{Deserialize, Serialize};

pub mod client;
pub mod metrics;
mod types;

struct DisplayStatus<'a>(&'a tonic::Status);

impl fmt::Display for DisplayStatus<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let s = &self.0;
		write!(f, "status: {:?}, message: {:?}", s.code(), s.message())?;

		if s.message().to_string().contains("authentication failure") {
			write!(
				f,
				" (hint: check the control plane logs for more information)"
			)?;
		}
		if !s.details().is_empty() {
			if let Ok(st) = std::str::from_utf8(s.details()) {
				write!(f, ", details: {st}")?;
			}
		}
		if let Some(src) = s.source().and_then(|s| s.source()) {
			write!(f, ", source: {src}")?;
			// Error is not public to explicitly match on, so do a fuzzy match
			if format!("{src}").contains("Temporary failure in name resolution") {
				write!(f, " (hint: is the DNS server reachable?)")?;
			}
		}
		Ok(())
	}
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("gRPC error {}", DisplayStatus(.0))]
	GrpcStatus(#[from] tonic::Status),
	#[error("gRPC connection error connecting to {}: {}", .0, DisplayStatus(.1))]
	Connection(String, #[source] tonic::Status),
	/// Attempted to send on a MPSC channel which has been canceled
	#[error(transparent)]
	RequestFailure(#[from] Box<mpsc::error::SendError<DeltaDiscoveryRequest>>),
	#[error("transport error: {0}")]
	Transport(#[from] tonic::transport::Error),
	// #[error("failed to send on demand resource")]
	// OnDemandSend(),
	// #[error("TLS Error: {0}")]
	// TLSError(#[from] tls::Error),
}

/// Updates the [ProxyState] from XDS.
/// All state updates code goes in ProxyStateUpdateMutator, that takes state as a parameter.
/// this guarantees that the state is always locked when it is updated.
#[derive(Clone)]
pub struct ProxyStateUpdateMutator {}

#[derive(Clone)]
pub struct ProxyStateUpdater {
	state: Arc<tokio::sync::RwLock<XdsStore>>,
	updater: ProxyStateUpdateMutator,
}

impl ProxyStateUpdater {
	/// Creates a new updater for the given stores. Will prefetch certs when workloads are updated.
	pub fn new(state: Arc<tokio::sync::RwLock<XdsStore>>) -> Self {
		Self {
			state,
			updater: ProxyStateUpdateMutator {},
		}
	}
}

impl ProxyStateUpdateMutator {
	#[instrument(
        level = Level::TRACE,
        name="insert_mcp_target",
        skip_all,
        fields(name=%proto.name),
    )]
	pub fn insert_mcp_target(&self, state: &mut XdsStore, proto: McpXdsTarget) -> anyhow::Result<()> {
		state.mcp_targets.insert(proto)?;
		Ok(())
	}

	#[instrument(
        level = Level::TRACE,
        name="remove_mcp_target",
        skip_all,
        fields(name=%xds_name),
    )]
	pub fn remove_mcp_target(&self, state: &mut XdsStore, xds_name: &Strng) -> anyhow::Result<()> {
		state.mcp_targets.remove(xds_name)?;
		Ok(())
	}

	#[instrument(
        level = Level::TRACE,
        name="insert_a2a_target",
        skip_all,
        fields(name=%proto.name),
    )]
	pub fn insert_a2a_target(&self, state: &mut XdsStore, proto: A2aXdsTarget) -> anyhow::Result<()> {
		state.a2a_targets.insert(proto)?;
		Ok(())
	}

	#[instrument(
        level = Level::TRACE,
        name="remove_a2a_target",
        skip_all,
        fields(name=%xds_name),
    )]
	pub fn remove_a2a_target(&self, state: &mut XdsStore, xds_name: &Strng) -> anyhow::Result<()> {
		state.a2a_targets.remove(xds_name)?;
		Ok(())
	}

	pub async fn insert_listener(
		&self,
		state: &mut XdsStore,
		listener: XdsListener,
	) -> anyhow::Result<()> {
		state.listeners.insert(listener).await
	}

	#[instrument(
        level = Level::TRACE,
        name="remove_listener",
        skip_all,
        fields(name=%xds_name),
    )]
	pub async fn remove_listener(
		&self,
		state: &mut XdsStore,
		xds_name: &Strng,
	) -> anyhow::Result<()> {
		state.listeners.remove(xds_name).await?;
		Ok(())
	}
}

#[async_trait::async_trait]
impl Handler<McpXdsTarget> for ProxyStateUpdater {
	async fn handle(&self, updates: Vec<XdsUpdate<McpXdsTarget>>) -> Result<(), Vec<RejectedConfig>> {
		let handle = |res: XdsUpdate<McpXdsTarget>| async {
			let mut state = self.state.write().await;
			match res {
				XdsUpdate::Update(w) => self.updater.insert_mcp_target(&mut state, w.resource)?,
				XdsUpdate::Remove(name) => self.updater.remove_mcp_target(&mut state, &name)?,
			}
			Ok(())
		};
		handle_single_resource(updates, handle).await
	}
}

#[async_trait::async_trait]
impl Handler<A2aXdsTarget> for ProxyStateUpdater {
	async fn handle(&self, updates: Vec<XdsUpdate<A2aXdsTarget>>) -> Result<(), Vec<RejectedConfig>> {
		let handle = |res: XdsUpdate<A2aXdsTarget>| async {
			let mut state = self.state.write().await;
			match res {
				XdsUpdate::Update(w) => self.updater.insert_a2a_target(&mut state, w.resource)?,
				XdsUpdate::Remove(name) => self.updater.remove_a2a_target(&mut state, &name)?,
			}
			Ok(())
		};
		handle_single_resource(updates, handle).await
	}
}

#[async_trait::async_trait]
impl Handler<XdsListener> for ProxyStateUpdater {
	async fn handle(&self, updates: Vec<XdsUpdate<XdsListener>>) -> Result<(), Vec<RejectedConfig>> {
		let handle = |res: XdsUpdate<XdsListener>| async {
			let mut state = self.state.write().await;
			match res {
				XdsUpdate::Update(w) => self.updater.insert_listener(&mut state, w.resource).await?,
				XdsUpdate::Remove(name) => self.updater.remove_listener(&mut state, &name).await?,
			}
			Ok(())
		};
		handle_single_resource(updates, handle).await
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
	#[error("missing fields")]
	MissingFields,
	#[error("invalid schema")]
	InvalidSchema,
}

impl TryFrom<XdsAuth> for Option<outbound::backend::BackendAuthConfig> {
	type Error = ParseError;
	fn try_from(value: XdsAuth) -> Result<Self, Self::Error> {
		match value.auth {
			Some(XdsAuthSpec::Passthrough(_)) => {
				Ok(Some(outbound::backend::BackendAuthConfig::Passthrough))
			},
			_ => Ok(None),
		}
	}
}

#[derive(Clone)]
pub struct TargetStore<T, M: prost::Message + Serialize + Clone> {
	by_name: HashMap<String, (outbound::Target<T>, tokio_util::sync::CancellationToken)>,

	by_name_protos: HashMap<String, M>,

	broadcast_tx: tokio::sync::broadcast::Sender<String>,
}

impl<T: Clone, M: prost::Message + Serialize + Clone> Serialize for TargetStore<T, M>
where
	outbound::Target<T>: Serialize,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self
			.by_name_protos
			.values()
			.cloned()
			.collect::<Vec<_>>()
			.serialize(serializer)
	}
}

impl<T, M: prost::Message + Serialize + Clone> Default for TargetStore<T, M>
where
	outbound::Target<T>: TryFrom<M, Error = anyhow::Error>,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<T, M: prost::Message + Serialize + Clone> TargetStore<T, M>
where
	outbound::Target<T>: TryFrom<M, Error = anyhow::Error>,
{
	pub fn new() -> Self {
		let (tx, _rx) = tokio::sync::broadcast::channel(16);
		Self {
			by_name: HashMap::new(),
			by_name_protos: HashMap::new(),
			broadcast_tx: tx,
		}
	}

	pub fn remove(
		&mut self,
		name: &str,
	) -> Result<(), tokio::sync::broadcast::error::SendError<String>> {
		if let Some((_target, ct)) = self.by_name.remove(name) {
			ct.cancel();
		}
		self.by_name_protos.remove(name);
		// Ignore the error, it's not critical, it just means there are clients using this target.
		let _ = self.broadcast_tx.send(name.to_string());
		Ok(())
	}

	#[instrument(
        level = Level::INFO,
        name="insert_target",
        skip_all,
    )]
	pub fn insert(&mut self, proto: M) -> anyhow::Result<()> {
		let converted_target: outbound::Target<T> = proto.clone().try_into()?;
		let ct = tokio_util::sync::CancellationToken::new();
		let name = converted_target.name.clone();
		self.by_name.insert(name.clone(), (converted_target, ct));
		self.by_name_protos.insert(name.clone(), proto);
		tracing::info!("inserted target: {}", name);
		Ok(())
	}

	pub fn get(
		&self,
		name: &str,
		listener_name: &str,
	) -> Option<(&outbound::Target<T>, &tokio_util::sync::CancellationToken)> {
		let target = self.by_name.get(name).map(|(target, ct)| (target, ct));
		if let Some((target, ct)) = target {
			if target.listeners.contains(&listener_name.to_string()) || target.listeners.is_empty() {
				return Some((target, ct));
			}
		}
		None
	}

	pub fn get_proto(&self, name: &str) -> Option<&M> {
		self.by_name_protos.get(name)
	}

	pub fn iter(
		&self,
		listener_name: &str,
	) -> impl Iterator<
		Item = (
			String,
			&(outbound::Target<T>, tokio_util::sync::CancellationToken),
		),
	> {
		self
			.by_name
			.iter()
			.filter(move |(_, target)| {
				target.0.listeners.contains(&listener_name.to_string()) || target.0.listeners.is_empty()
			})
			.map(|(name, target)| (name.clone(), target))
	}

	pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<String> {
		self.broadcast_tx.subscribe()
	}
}

#[derive(Clone, Deserialize)]
pub struct PolicyStore {
	by_name: HashMap<String, rbac::RuleSet>,
	by_name_protos: HashMap<String, XdsRbac>,
}

impl Serialize for PolicyStore {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.by_name_protos.serialize(serializer)
	}
}

impl PolicyStore {
	pub fn new() -> Self {
		Self {
			by_name: HashMap::new(),
			by_name_protos: HashMap::new(),
		}
	}
}

impl Default for PolicyStore {
	fn default() -> Self {
		Self::new()
	}
}

impl PolicyStore {
	pub fn insert(&mut self, policy: XdsRbac) -> anyhow::Result<()> {
		let policy_name = policy.name.clone();
		let rule_set = rbac::RuleSet::try_from(&policy)?;
		self.by_name.insert(policy_name.clone(), rule_set);
		self.by_name_protos.insert(policy_name, policy);
		Ok(())
	}

	pub fn get_proto(&self, name: &str) -> Option<&XdsRbac> {
		self.by_name_protos.get(name)
	}

	pub fn remove(&mut self, name: &str) {
		self.by_name.remove(name);
	}

	pub fn validate(&self, resource: &rbac::ResourceType, claims: &rbac::Identity) -> bool {
		self
			.by_name
			.values()
			.any(|policy| policy.validate(resource, claims))
	}

	pub fn clear(&mut self) {
		self.by_name.clear();
	}
}

#[derive(Clone)]
pub struct ListenerStore {
	by_name: HashMap<String, inbound::Listener>,
	by_name_protos: HashMap<String, XdsListener>,
	update_tx: tokio::sync::mpsc::Sender<UpdateEvent>,
}

impl Serialize for ListenerStore {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self
			.by_name_protos
			.values()
			.cloned()
			.collect::<Vec<_>>()
			.serialize(serializer)
	}
}
pub enum UpdateEvent {
	Insert(String),
	Update(String),
	Remove(String),
}

impl ListenerStore {
	pub fn new(update_tx: tokio::sync::mpsc::Sender<UpdateEvent>) -> Self {
		Self {
			by_name: HashMap::new(),
			by_name_protos: HashMap::new(),
			update_tx,
		}
	}
}

impl ListenerStore {
	pub fn iter(&self) -> impl Iterator<Item = (&String, &inbound::Listener)> {
		self.by_name.iter()
	}

	pub async fn insert(&mut self, listener: XdsListener) -> anyhow::Result<()> {
		let listener_name = listener.name.clone();
		self
			.by_name_protos
			.insert(listener_name.clone(), listener.clone());
		let xds_listener = inbound::Listener::from_xds(listener).await?;
		if self.by_name.iter().any(|(_, listener)| {
			// Return true if the incoming listener binds to the same address as an existing listener.
			// Unless the names are the same, in which case we return false, because we will update the existing listener.
			match (&listener.spec, &xds_listener.spec) {
				(inbound::ListenerType::Sse(l), inbound::ListenerType::Sse(r)) => {
					l.addr == r.addr && listener.name != listener_name
				},
				(inbound::ListenerType::A2a(l), inbound::ListenerType::A2a(r)) => {
					l.addr == r.addr && listener.name != listener_name
				},
				_ => false,
			}
		}) {
			return Err(anyhow::anyhow!(
				"listener already exists which binds to the same address: {}",
				listener_name
			));
		}
		match self.by_name.insert(listener_name.clone(), xds_listener) {
			Some(_) => {
				self
					.update_tx
					.send(UpdateEvent::Update(listener_name))
					.await
					.map_err(|e| anyhow::anyhow!("failed to send update event: {:?}", e))?;
			},
			None => {
				self
					.update_tx
					.send(UpdateEvent::Insert(listener_name))
					.await
					.map_err(|e| anyhow::anyhow!("failed to send update event: {:?}", e))?;
			},
		}
		Ok(())
	}

	pub fn get(&self, listener_name: &str) -> Option<&inbound::Listener> {
		self.by_name.get(listener_name)
	}

	pub fn get_proto(&self, listener_name: &str) -> Option<&XdsListener> {
		self.by_name_protos.get(listener_name)
	}

	pub async fn remove(&mut self, listener_name: &str) -> anyhow::Result<()> {
		self.by_name_protos.remove(listener_name);
		self.by_name.remove(listener_name);
		self
			.update_tx
			.send(UpdateEvent::Remove(listener_name.to_string()))
			.await
			.map_err(|e| anyhow::anyhow!("failed to send update event: {:?}", e))?;
		Ok(())
	}
}

pub struct XdsStore {
	pub a2a_targets: TargetStore<outbound::A2aTargetSpec, A2aXdsTarget>,
	pub mcp_targets: TargetStore<outbound::McpTargetSpec, McpXdsTarget>,
	pub listeners: ListenerStore,
}

impl XdsStore {
	pub fn new(update_tx: tokio::sync::mpsc::Sender<UpdateEvent>) -> Self {
		Self {
			a2a_targets: TargetStore::new(),
			mcp_targets: TargetStore::new(),
			listeners: ListenerStore::new(update_tx),
		}
	}
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
	pub xds_address: String,
	pub metadata: HashMap<String, String>,
	pub listeners: Vec<XdsListener>,
}
