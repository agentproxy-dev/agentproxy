use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use agent_core::prelude::*;
use agent_xds::{XdsResource, XdsUpdate};
use itertools::Itertools;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::Debouncer;
use serde::{Deserialize, Deserializer};
use tokio::sync::RwLock;

use crate::client::Client;
use crate::store::{LocalWorkload, Stores};
use crate::types::agent::{
	Bind, BindName, Listener, ListenerProtocol, ListenerSet, Policy, PolicyName, Route, RouteSet,
	TLSConfig, TargetedPolicy,
};
use crate::types::discovery::{NamespacedHostname, Service, Workload};
use crate::types::proto::agent::Resource as ADPResource;
use crate::types::proto::workload::{
	Address as XdsAddress, GatewayAddress as XdsGatewayAddress, LoadBalancing as XdsLoadBalancing,
	Locality as XdsLocality, NetworkAddress as XdsNetworkAddress, Port as XdsPort, PortList,
	Service as XdsService, Workload as XdsWorkload, address, gateway_address,
};
use crate::{ConfigSource, client, serdes, store};

#[derive(serde::Serialize)]
pub struct StateManager {
	#[serde(flatten)]
	stores: Stores,

	#[serde(skip_serializing)]
	xds_client: Option<agent_xds::AdsClient>,
}

pub const ADDRESS_TYPE: Strng =
	strng::literal!("type.googleapis.com/agentgateway.dev.workload.Address");
pub const AUTHORIZATION_TYPE: Strng =
	strng::literal!("type.googleapis.com/istio.security.Authorization");
pub const ADP_TYPE: Strng =
	strng::literal!("type.googleapis.com/agentgateway.dev.resource.Resource");

impl StateManager {
	pub async fn new(
		config: &crate::XDSConfig,
		client: client::Client,
		xds_metrics: agent_xds::Metrics,
		awaiting_ready: tokio::sync::watch::Sender<()>,
	) -> anyhow::Result<Self> {
		let stores = Stores::new();

		let xds_client = if config.address.is_some() {
			Some(
				agent_xds::Config::new(
					config.address.as_ref().unwrap().clone(),
					config.gateway.clone(),
					config.namespace.clone(),
				)
				.with_watched_handler::<XdsAddress>(ADDRESS_TYPE, stores.clone().discovery.clone())
				.with_watched_handler::<ADPResource>(ADP_TYPE, stores.clone().binds.clone())
				// .with_watched_handler::<XdsAuthorization>(AUTHORIZATION_TYPE, state)
				.build(xds_metrics, awaiting_ready),
			)
		} else {
			None
		};
		if let Some(cfg) = &config.local_config {
			let local_client = LocalClient {
				stores: stores.clone(),
				cfg: cfg.clone(),
				client,
			};
			local_client.run().await?;
		}
		Ok(Self { stores, xds_client })
	}

	pub fn stores(&self) -> Stores {
		self.stores.clone()
	}

	pub async fn run(self) -> anyhow::Result<()> {
		match self.xds_client {
			Some(xds) => xds.run().await.map_err(|e| anyhow::anyhow!(e)),
			None => Ok(()),
		}
	}
}

/// LocalClient serves as a local file reader alternative for XDS. This is intended for testing.
#[derive(Debug, Clone)]
pub struct LocalClient {
	pub cfg: ConfigSource,
	pub stores: Stores,
	pub client: Client,
}

impl LocalClient {
	pub async fn run(self) -> Result<(), anyhow::Error> {
		if let ConfigSource::File(path) = &self.cfg {
			// Load initial state then watch
			self.watch_config_file(path).await?;
		} else {
			// Load it once
			self.reload_config(PreviousState::default()).await?;
		}

		Ok(())
	}

	async fn watch_config_file(&self, path: &Path) -> anyhow::Result<()> {
		let (tx, mut rx) = tokio::sync::mpsc::channel(1);

		// Create a watcher with a 250ms debounce
		let mut watcher =
			notify_debouncer_full::new_debouncer(Duration::from_millis(250), None, move |res| {
				futures::executor::block_on(async {
					tx.send(res).await.unwrap();
				})
			})
			.map_err(|e| anyhow::anyhow!("Failed to create file watcher: {}", e))?;

		// Watch the config file
		watcher
			.watch(path, RecursiveMode::NonRecursive)
			.map_err(|e| anyhow::anyhow!("Failed to watch config file: {}", e))?;

		info!("Watching config file: {}", path.display());

		let lc: LocalClient = self.to_owned();
		let mut next_state = lc.reload_config(PreviousState::default()).await?;
		tokio::task::spawn(async move {
			use notify_debouncer_full::DebouncedEvent;
			// Handle file change events
			while let Some(Ok(events)) = rx.recv().await {
				// Only process if we have actual content changes
				if events
					.iter()
					.any(|e| matches!(e.kind, EventKind::Modify(_) | EventKind::Create(_)))
				{
					info!("Config file changed, reloading...");
					match lc.reload_config(next_state.clone()).await {
						Ok(nxt) => {
							next_state = nxt;
							info!("Config reloaded successfully")
						},
						Err(e) => {
							error!("Failed to reload config: {}", e)
						},
					}
				}
			}
			drop(watcher);
		});

		Ok(())
	}

	async fn reload_config(&self, prev: PreviousState) -> anyhow::Result<PreviousState> {
		let config_content = self.cfg.read_to_string().await?;
		let config = crate::types::local::NormalizedLocalConfig::from(
			self.client.clone(),
			config_content.as_str(),
		)
		.await?;
		info!("loaded config from {:?}", self.cfg);

		// Sync the state
		let next_binds =
			self
				.stores
				.binds
				.sync_local(config.binds, config.policies, config.backends, prev.binds);
		let next_discovery =
			self
				.stores
				.discovery
				.sync_local(config.services, config.workloads, prev.discovery);

		Ok(PreviousState {
			binds: next_binds,
			discovery: next_discovery,
		})
	}
}

#[derive(Clone, Debug, Default)]
pub struct PreviousState {
	pub binds: store::BindPreviousState,
	pub discovery: store::DiscoveryPreviousState,
}
