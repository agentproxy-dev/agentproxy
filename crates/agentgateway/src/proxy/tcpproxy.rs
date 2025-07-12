use crate::client::Transport;
use crate::http::Request;
use crate::proxy::ProxyError;
use crate::telemetry::log::RequestLog;
use crate::transport::stream;
use crate::transport::stream::{Socket, TCPConnectionInfo, TLSConnectionInfo};
use crate::types::agent;
use crate::types::agent::{
	Backend, BackendReference, BindName, HeaderMatch, HeaderValueMatch, Listener, ListenerProtocol,
	PathMatch, PolicyTarget, QueryValueMatch, Route, RouteBackend, RouteBackendReference,
	SimpleBackend, SimpleBackendReference, TCPRoute, TCPRouteBackend, TCPRouteBackendReference,
	Target,
};
use crate::types::discovery::NetworkAddress;
use crate::types::discovery::gatewayaddress::Destination;
use crate::{ProxyInputs, *};
use agent_core::strng;
use anyhow::anyhow;
use itertools::Itertools;
use rand::prelude::IndexedRandom;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct TCPProxy {
	pub(super) bind_name: BindName,
	pub(super) inputs: Arc<ProxyInputs>,
	pub(super) selected_listener: Arc<Listener>,
	pub(super) target_address: SocketAddr,
}

impl TCPProxy {
	pub async fn proxy(&self, connection: Socket) {
		let mut log: RequestLog = Default::default();
		let ret = self.proxy_internal(connection, &mut log).await;
		if let Err(e) = ret {
			log.error = Some(e.to_string());
		}
	}

	async fn proxy_internal(
		&self,
		connection: Socket,
		log: &mut RequestLog,
	) -> Result<(), ProxyError> {
		let start = Instant::now();
		log.start = Some(start);
		log.tcp_info = connection.ext::<TCPConnectionInfo>().cloned();
		log.tls_info = connection.ext::<TLSConnectionInfo>().cloned();
		let sni = log
			.tls_info
			.as_ref()
			.and_then(|tls| tls.server_name.as_deref());
		// log.metrics = Some(self.inputs.metrics.clone());

		let selected_listener = self.selected_listener.clone();
		let upstream = self.inputs.upstream.clone();
		let inputs = self.inputs.clone();
		let bind_name = self.bind_name.clone();
		debug!(bind=%bind_name, "route for bind");
		log.gateway_name = Some(selected_listener.gateway_name.clone());
		log.listener_name = Some(selected_listener.name.clone());
		debug!(bind=%bind_name, listener=%selected_listener.key, "selected listener");

		let (selected_route) =
			select_best_route(sni, selected_listener.clone()).ok_or(ProxyError::RouteNotFound)?;
		log.route_rule_name = selected_route.rule_name.clone();
		log.route_name = Some(selected_route.route_name.clone());

		debug!(bind=%bind_name, listener=%selected_listener.key, route=%selected_route.key, "selected route");
		let selected_backend =
			select_tcp_backend(selected_route.as_ref()).ok_or(ProxyError::NoValidBackends)?;
		let selected_backend = resolve_backend(selected_backend, self.inputs.as_ref())?;

		let (target, policy_key) = match &selected_backend.backend {
			SimpleBackend::Service(_, _) => {
				return Err(ProxyError::Processing(anyhow!(
					"service is not currently supported for TCPRoute"
				)));
			},
			SimpleBackend::Opaque(name, target) => (target.clone(), PolicyTarget::Backend(name.clone())),
			SimpleBackend::Invalid => return Err(ProxyError::BackendDoesNotExist),
		};

		let policies = inputs
			.stores
			.read_binds()
			.backend_policies(policy_key.clone());
		// let transport = policies.i // TODO
		let transport = Transport::Plaintext;
		let Target::Address(addr) = target else {
			panic!("TODO")
		};
		let upstream = stream::Socket::dial(addr)
			.await
			.map_err(ProxyError::Processing)?;
		agent_core::copy::copy_bidirectional(
			connection,
			upstream,
			&agent_core::copy::ConnectionResult {},
		)
		.await
		.map_err(|e| ProxyError::Processing(e.into()))?;
		Ok(())
	}
}

fn select_best_route(host: Option<&str>, listener: Arc<Listener>) -> Option<Arc<TCPRoute>> {
	// TCP matching is much simpler than HTTP.
	// We pick the best matching hostname, else fallback to precedence:
	//
	//  * The oldest Route based on creation timestamp.
	//  * The Route appearing first in alphabetical order by "{namespace}/{name}".

	// Assume matches are ordered already (not true today)
	if matches!(listener.protocol, ListenerProtocol::HBONE) && listener.routes.is_empty() {
		// TODO: TCP for waypoint
		return None;
	}
	for hnm in agent::HostnameMatch::all_matches_or_none(host) {
		if let Some(r) = listener.tcp_routes.get_hostname(&hnm) {
			return Some(Arc::new(r.clone()));
		}
	}
	None
}

fn select_tcp_backend(route: &TCPRoute) -> Option<TCPRouteBackendReference> {
	route
		.backends
		.choose_weighted(&mut rand::rng(), |b| b.weight)
		.ok()
		.cloned()
}

fn resolve_backend(
	b: TCPRouteBackendReference,
	pi: &ProxyInputs,
) -> Result<TCPRouteBackend, ProxyError> {
	let backend = super::resolve_simple_backend(&b.backend, pi)?;
	Ok(TCPRouteBackend {
		weight: b.weight,
		backend,
	})
}
