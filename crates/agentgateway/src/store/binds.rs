use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use agent_xds::{RejectedConfig, XdsUpdate};
use futures_core::Stream;
use itertools::Itertools;
use serde::Serialize;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tracing::{Level, instrument};

use crate::http::auth::BackendAuth;
use crate::http::backendtls::BackendTLS;
use crate::http::{ext_authz, remoteratelimit};
use crate::mcp::rbac::{RuleSet, RuleSets};
use crate::store::Event;
use crate::types::agent::{
	A2aPolicy, Backend, BackendName, Bind, BindName, GatewayName, Listener, ListenerKey, ListenerSet,
	McpAuthentication, Policy, PolicyName, PolicyTarget, Route, RouteKey, RouteName, TargetedPolicy,
};
use crate::types::discovery::{NamespacedHostname, Service, Workload};
use crate::types::proto::agent::resource::Kind as XdsKind;
use crate::types::proto::agent::{
	Bind as XdsBind, Listener as XdsListener, Resource as ADPResource, Route as XdsRoute,
};
use crate::*;

#[derive(Debug)]
pub struct Store {
	/// Allows for lookup of services by network address, the service's xds secondary key.
	by_name: HashMap<BindName, Arc<Bind>>,

	policies_by_name: HashMap<PolicyName, Arc<TargetedPolicy>>,

	backends_by_name: HashMap<BackendName, Arc<Backend>>,

	tx: tokio::sync::broadcast::Sender<Event<Arc<Bind>>>,
}

#[derive(Default, Debug, Clone)]
pub struct BackendPolicies {
	pub backend_tls: Option<BackendTLS>,
	pub backend_auth: Option<BackendAuth>,
	pub a2a: Option<A2aPolicy>,
	// bool represents "should use default settings for provider"
	pub llm_provider: Option<(llm::AIProvider, bool)>,
	pub llm: Option<llm::Policy>,
}

impl BackendPolicies {
	// Merges self and other. Other has precedence
	pub fn merge(self, other: BackendPolicies) -> BackendPolicies {
		Self {
			backend_tls: other.backend_tls.or(self.backend_tls),
			backend_auth: other.backend_auth.or(self.backend_auth),
			a2a: other.a2a.or(self.a2a),
			llm: other.llm.or(self.llm),
			llm_provider: other.llm_provider.or(self.llm_provider),
		}
	}
}

#[derive(Debug)]
pub struct RoutePolicies {
	pub local_rate_limit: Vec<http::localratelimit::RateLimit>,
	pub remote_rate_limit: Option<remoteratelimit::RemoteRateLimit>,
	pub jwt: Option<http::jwt::Jwt>,
	pub ext_authz: Option<ext_authz::ExtAuthz>,
}

impl From<RoutePolicies> for LLMRoutePolicies {
	fn from(value: RoutePolicies) -> Self {
		LLMRoutePolicies {
			local_rate_limit: value
				.local_rate_limit
				.iter()
				.filter(|r| r.limit_type == http::localratelimit::RateLimitType::Tokens)
				.cloned()
				.collect(),
		}
	}
}

#[derive(Debug, Default)]
pub struct LLMRoutePolicies {
	pub local_rate_limit: Vec<http::localratelimit::RateLimit>,
}

impl Default for Store {
	fn default() -> Self {
		Self::new()
	}
}
impl Store {
	pub fn new() -> Self {
		let (tx, _) = tokio::sync::broadcast::channel(1000);
		Self {
			by_name: Default::default(),
			policies_by_name: Default::default(),
			backends_by_name: Default::default(),
			tx,
		}
	}
	pub fn subscribe(
		&self,
	) -> (impl Stream<Item = Result<Event<Arc<Bind>>, BroadcastStreamRecvError>> + use<>) {
		let sub = self.tx.subscribe();
		tokio_stream::wrappers::BroadcastStream::new(sub)
	}

	pub fn route_policies(
		&self,
		route_rule: RouteKey,
		route: RouteName,
		gateway: GatewayName,
	) -> RoutePolicies {
		let route_rule = PolicyTarget::RouteRule(route_rule);
		let route = PolicyTarget::Route(route);
		let gateway = PolicyTarget::Gateway(gateway);
		// Changes we must do:
		// * Index the store by the target
		// * Avoid the N lookups, or at least the boilerplate, for each type
		// Changes we may want to consider:
		// * We do this lookup under one lock, but we will lookup backend rules and listener rules under a different
		//   lock. This can lead to inconsistent state..
		let local_rate_limit = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				let tgt = &p.target;
				if !(tgt == &route || tgt == &route_rule || tgt == &gateway) {
					return None;
				}
				match &p.policy {
					Policy::LocalRateLimit(lrl) => Some(lrl.clone()),
					_ => None,
				}
			})
			.next();
		let jwt = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				let tgt = &p.target;
				if !(tgt == &route || tgt == &route_rule || tgt == &gateway) {
					return None;
				}
				match &p.policy {
					Policy::JwtAuth(lrl) => Some(lrl.clone()),
					_ => None,
				}
			})
			.next();
		let ext_authz = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				let tgt = &p.target;
				if !(tgt == &route || tgt == &route_rule || tgt == &gateway) {
					return None;
				}
				match &p.policy {
					Policy::ExtAuthz(lrl) => Some(lrl.clone()),
					_ => None,
				}
			})
			.next();
		let remote_rate_limit = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				let tgt = &p.target;
				if !(tgt == &route || tgt == &route_rule || tgt == &gateway) {
					return None;
				}
				match &p.policy {
					Policy::RemoteRateLimit(lrl) => Some(lrl.clone()),
					_ => None,
				}
			})
			.next();
		RoutePolicies {
			local_rate_limit: local_rate_limit.unwrap_or_default(),
			remote_rate_limit,
			jwt,
			ext_authz,
		}
	}

	pub fn backend_policies(&self, tgt: PolicyTarget) -> BackendPolicies {
		let tls = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				if p.target != tgt {
					return None;
				};
				match &p.policy {
					Policy::BackendTLS(btls) => Some(btls.clone()),
					_ => None,
				}
			})
			.next();
		let auth = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				if p.target != tgt {
					return None;
				};
				match &p.policy {
					Policy::BackendAuth(ba) => Some(ba.clone()),
					_ => None,
				}
			})
			.next();
		let a2a = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				if p.target != tgt {
					return None;
				};
				match &p.policy {
					Policy::A2a(ba) => Some(ba.clone()),
					_ => None,
				}
			})
			.next();
		let llm = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				if p.target != tgt {
					return None;
				};
				match &p.policy {
					Policy::AI(ba) => Some(ba.clone()),
					_ => None,
				}
			})
			.next();
		BackendPolicies {
			backend_tls: tls,
			backend_auth: auth,
			a2a,
			llm,
			// These are not attached policies but are represented in this struct for code organization
			llm_provider: None,
		}
	}

	pub fn mcp_policies(&self, backend: BackendName) -> (RuleSets, Option<McpAuthentication>) {
		let t = PolicyTarget::Backend(backend);
		let rs = RuleSets::from(
			self
				.policies_by_name
				.values()
				.filter_map(|p| {
					if p.target != t {
						return None;
					};
					match &p.policy {
						Policy::McpAuthorization(authz) => Some(authz.clone().into_inner()),
						_ => None,
					}
				})
				.collect_vec(),
		);
		let auth = self
			// This is a terrible approach!
			.policies_by_name
			.values()
			.filter_map(|p| {
				if p.target != t {
					return None;
				};
				match &p.policy {
					Policy::McpAuthentication(ba) => Some(ba.clone()),
					_ => None,
				}
			})
			.next();
		(rs, auth)
	}

	pub fn listeners(&self, bind: BindName) -> Option<ListenerSet> {
		// TODO: clone here is terrible!!!
		self.by_name.get(&bind).map(|b| b.listeners.clone())
	}

	pub fn all(&self) -> Vec<Arc<Bind>> {
		self.by_name.values().cloned().collect()
	}

	pub fn backend(&self, r: &BackendName) -> Option<Arc<Backend>> {
		self.backends_by_name.get(r).cloned()
	}

	#[instrument(
        level = Level::INFO,
        name="remove_bind",
        skip_all,
        fields(bind),
    )]
	pub fn remove_bind(&mut self, bind: BindName) {
		if let Some(old) = self.by_name.remove(&bind) {
			let _ = self.tx.send(Event::Remove(old));
		}
	}
	#[instrument(
        level = Level::INFO,
        name="remove_policy",
        skip_all,
        fields(bind),
    )]
	pub fn remove_policy(&mut self, pol: PolicyName) {
		if let Some(old) = self.policies_by_name.remove(&pol) {}
	}
	#[instrument(
        level = Level::INFO,
        name="remove_backend",
        skip_all,
        fields(bind),
    )]
	pub fn remove_backend(&mut self, backend: BackendName) {
		if let Some(old) = self.backends_by_name.remove(&backend) {}
	}

	#[instrument(
        level = Level::INFO,
        name="remove_listener",
        skip_all,
        fields(listener),
    )]
	pub fn remove_listener(&mut self, listener: ListenerKey) {
		let Some(bind) = self
			.by_name
			.values()
			.find(|v| v.listeners.contains(&listener))
		else {
			return;
		};
		let mut bind = Arc::unwrap_or_clone(bind.clone());
		bind.listeners.remove(&listener);
		self.insert_bind(bind);
	}

	#[instrument(
        level = Level::INFO,
        name="remove_route",
        skip_all,
        fields(route),
    )]
	pub fn remove_route(&mut self, route: RouteKey) {
		let Some((_, bind, listener)) = self.by_name.iter().find_map(|(k, v)| {
			let l = v.listeners.iter().find(|l| l.routes.contains(&route));
			l.map(|l| (k.clone(), v.clone(), l.clone()))
		}) else {
			return;
		};
		let mut bind = Arc::unwrap_or_clone(bind.clone());
		let ln = listener.key.clone();
		let mut lis = listener.clone();
		lis.routes.remove(&route);
		bind.listeners.insert(ln, lis);
		self.insert_bind(bind);
	}

	#[instrument(
        level = Level::INFO,
        name="insert_bind",
        skip_all,
        fields(bind=%bind.key),
    )]
	pub fn insert_bind(&mut self, bind: Bind) {
		// TODO: handle update
		let arc = Arc::new(bind);
		self.by_name.insert(arc.key.clone(), arc.clone());
		// ok to have no subs
		let _ = self.tx.send(Event::Add(arc));
	}

	pub fn insert_backend(&mut self, b: Backend) {
		// TODO: handle update
		let name = b.name();
		let arc = Arc::new(b);
		self.backends_by_name.insert(name, arc);
	}

	#[instrument(
        level = Level::INFO,
        name="insert_policy",
        skip_all,
        fields(pol=%pol.name),
    )]
	pub fn insert_policy(&mut self, pol: TargetedPolicy) {
		// TODO: handle update
		let arc = Arc::new(pol);
		self.policies_by_name.insert(arc.name.clone(), arc.clone());
	}

	#[instrument(
        level = Level::INFO,
        name="insert_listener",
        skip_all,
        fields(listener=%lis.name,bind=%bind_name),
    )]
	pub fn insert_listener(&mut self, lis: Listener, bind_name: BindName) {
		if let Some(b) = self.by_name.get(&bind_name) {
			let mut bind = Arc::unwrap_or_clone(b.clone());
			bind.listeners.insert(lis.key.clone(), lis);
			self.insert_bind(bind);
		} else {
			warn!("no bind found");
		}
	}
	#[instrument(
        level = Level::INFO,
        name="insert_route",
        skip_all,
        fields(listener=%ln,route=%r.key),
    )]
	pub fn insert_route(&mut self, r: Route, ln: ListenerKey) {
		let Some((bind, lis)) = self
			.by_name
			.values()
			.find_map(|l| l.listeners.get(&ln).map(|ls| (l, ls)))
		else {
			warn!("no listener found");
			return;
		};
		let mut bind = Arc::unwrap_or_clone(bind.clone());
		let mut lis = lis.clone();
		lis.routes.insert(r);
		bind.listeners.insert(ln, lis);
		self.insert_bind(bind);
	}

	fn remove_resource(&mut self, res: &Strng) {
		trace!("removing res {res}...");
		let Some((res, res_name)) = res.split_once("/") else {
			trace!("unknown resource name {res}");
			return;
		};
		match res {
			"bind" => {
				self.remove_bind(strng::new(res_name));
			},
			"listener" => {
				self.remove_listener(strng::new(res_name));
			},
			"route" => {
				self.remove_route(strng::new(res_name));
			},
			_ => {
				error!("unknown resource kind {res}");
			},
		}
	}

	fn insert_xds(&mut self, res: ADPResource) -> anyhow::Result<()> {
		trace!("insert resource {res:?}");
		match res.kind {
			Some(XdsKind::Bind(w)) => self.insert_xds_bind(w),
			Some(XdsKind::Listener(w)) => self.insert_xds_listener(w),
			Some(XdsKind::Route(w)) => self.insert_xds_route(w),
			_ => Err(anyhow::anyhow!("unknown resource type")),
		}
	}

	fn insert_xds_bind(&mut self, raw: XdsBind) -> anyhow::Result<()> {
		let bind = Bind::try_from(&raw)?;
		self.insert_bind(bind);
		Ok(())
	}
	fn insert_xds_listener(&mut self, raw: XdsListener) -> anyhow::Result<()> {
		let (lis, bind_name): (Listener, BindName) = (&raw).try_into()?;
		self.insert_listener(lis, bind_name);
		Ok(())
	}
	fn insert_xds_route(&mut self, raw: XdsRoute) -> anyhow::Result<()> {
		let (route, listener_name): (Route, ListenerKey) = (&raw).try_into()?;
		self.insert_route(route, listener_name);
		Ok(())
	}
}

#[derive(Clone, Debug)]
pub struct StoreUpdater {
	state: Arc<RwLock<Store>>,
}

#[derive(serde::Serialize)]
pub struct Dump {
	binds: Vec<Arc<Bind>>,
	policies: Vec<Arc<TargetedPolicy>>,
	backends: Vec<Arc<Backend>>,
}

impl StoreUpdater {
	pub fn new(state: Arc<RwLock<Store>>) -> StoreUpdater {
		Self { state }
	}
	pub fn read(&self) -> std::sync::RwLockReadGuard<'_, Store> {
		self.state.read().expect("mutex acquired")
	}
	pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, Store> {
		self.state.write().expect("mutex acquired")
	}
	pub fn dump(&self) -> Dump {
		let store = self.state.read().expect("mutex");
		// Services all have hostname, so use that as the key
		let binds: Vec<_> = store
			.by_name
			.iter()
			.sorted_by_key(|k| k.0)
			.map(|k| k.1.clone())
			.collect();
		let policies: Vec<_> = store
			.policies_by_name
			.iter()
			.sorted_by_key(|k| k.0)
			.map(|k| k.1.clone())
			.collect();
		let backends: Vec<_> = store
			.backends_by_name
			.iter()
			.sorted_by_key(|k| k.0)
			.map(|k| k.1.clone())
			.collect();
		Dump {
			binds,
			policies,
			backends,
		}
	}
	pub fn sync_local(
		&self,
		binds: Vec<Bind>,
		policies: Vec<TargetedPolicy>,
		backends: Vec<Backend>,
		prev: PreviousState,
	) -> PreviousState {
		let mut s = self.state.write().expect("mutex acquired");
		let mut old_binds = prev.binds;
		let mut old_pols = prev.policies;
		let mut old_backends = prev.backends;
		let mut next_state = PreviousState {
			binds: Default::default(),
			policies: Default::default(),
			backends: Default::default(),
		};
		for b in binds {
			old_binds.remove(&b.key);
			next_state.binds.insert(b.key.clone());
			s.insert_bind(b);
		}
		for b in backends {
			old_backends.remove(&b.name());
			next_state.backends.insert(b.name());
			s.insert_backend(b);
		}
		for p in policies {
			old_pols.remove(&p.name);
			next_state.policies.insert(p.name.clone());
			s.insert_policy(p);
		}
		for remaining_bind in old_binds {
			s.remove_bind(remaining_bind);
		}
		for remaining_policy in old_pols {
			s.remove_policy(remaining_policy);
		}
		for remaining_backend in old_backends {
			s.remove_backend(remaining_backend);
		}
		next_state
	}
}

#[derive(Clone, Debug, Default)]
pub struct PreviousState {
	pub binds: HashSet<BindName>,
	pub policies: HashSet<PolicyName>,
	pub backends: HashSet<BackendName>,
}

impl agent_xds::Handler<ADPResource> for StoreUpdater {
	fn handle(
		&self,
		updates: Box<&mut dyn Iterator<Item = XdsUpdate<ADPResource>>>,
	) -> Result<(), Vec<RejectedConfig>> {
		let mut state = self.state.write().unwrap();
		let handle = |res: XdsUpdate<ADPResource>| {
			match res {
				XdsUpdate::Update(w) => state.insert_xds(w.resource)?,
				XdsUpdate::Remove(name) => {
					debug!("handling delete {}", name);
					state.remove_resource(&strng::new(name))
				},
			}
			Ok(())
		};
		agent_xds::handle_single_resource(updates, handle)
	}
}
