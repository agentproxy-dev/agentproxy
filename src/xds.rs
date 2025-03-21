// Copyright Istio Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bytes::Bytes;
use std::error::Error as StdErr;
use std::fmt;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::Level;

use serde_yaml;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, trace, warn};

pub use client::*;
pub use metrics::*;
pub use types::*;

use xds::mcp::kgateway_dev::rbac::Config as XdsRbac;
use xds::mcp::kgateway_dev::target::Target as XdsTarget;

use crate::state::{Listener, State as ProxyState, Target};
use crate::strng::Strng;
use crate::xds;
use crate::{rbac};

use self::envoy::service::discovery::v3::DeltaDiscoveryRequest;

mod client;
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
	state: Arc<RwLock<ProxyState>>,
	updater: ProxyStateUpdateMutator,
}

impl ProxyStateUpdater {
	/// Creates a new updater for the given stores. Will prefetch certs when workloads are updated.
	pub fn new(state: Arc<RwLock<ProxyState>>) -> Self {
		Self {
			state,
			updater: ProxyStateUpdateMutator {},
		}
	}
}

impl ProxyStateUpdateMutator {
	#[instrument(
        level = Level::TRACE,
        name="insert_target",
        skip_all,
        fields(name=%target.name),
    )]
	pub fn insert_target(&self, state: &mut ProxyState, target: XdsTarget) -> anyhow::Result<()> {
		let target = Target::from(&target);
		state.targets.insert(target);
		Ok(())
	}

	#[instrument(
        level = Level::TRACE,
        name="remove_target",
        skip_all,
        fields(name=%xds_name),
    )]
	pub fn remove_target(&self, state: &mut ProxyState, xds_name: &Strng) {
		state.targets.remove(xds_name);
	}

	#[instrument(
        level = Level::TRACE,
        name="insert_rbac",
        skip_all,
    )]
	pub fn insert_rbac(&self, state: &mut ProxyState, rbac: XdsRbac) -> anyhow::Result<()> {
		let rule_set = rbac::RuleSet::from(&rbac);
		state.policies.insert(rule_set);
		Ok(())
	}

	#[instrument(
        level = Level::TRACE,
        name="remove_rbac",
        skip_all,
        fields(name=%xds_name),
    )]
	pub fn remove_rbac(&self, state: &mut ProxyState, xds_name: &Strng) {
		state.policies.remove(xds_name);
	}
}

impl Handler<XdsTarget> for ProxyStateUpdater {
	fn handle(
		&self,
		updates: Box<&mut dyn Iterator<Item = XdsUpdate<XdsTarget>>>,
	) -> Result<(), Vec<RejectedConfig>> {
		let mut state = self.state.write().unwrap();
		let handle = |res: XdsUpdate<XdsTarget>| {
			match res {
				XdsUpdate::Update(w) => self.updater.insert_target(&mut state, w.resource)?,
				XdsUpdate::Remove(name) => self.updater.remove_target(&mut state, &name),
			}
			Ok(())
		};
		handle_single_resource(updates, handle)
	}
}

impl Handler<XdsRbac> for ProxyStateUpdater {
	fn handle(
		&self,
		updates: Box<&mut dyn Iterator<Item = XdsUpdate<XdsRbac>>>,
	) -> Result<(), Vec<RejectedConfig>> {
		let mut state = self.state.write().unwrap();
		let handle = |res: XdsUpdate<XdsRbac>| {
			match res {
				XdsUpdate::Update(w) => self.updater.insert_rbac(&mut state, w.resource)?,
				XdsUpdate::Remove(name) => self.updater.remove_rbac(&mut state, &name),
			}
			Ok(())
		};
		handle_single_resource(updates, handle)
	}
}

/// LocalClient serves as a local file reader alternative for XDS. This is intended for testing.
pub struct LocalClient {
	pub cfg: LocalConfig,
	pub state: Arc<RwLock<ProxyState>>,
}

#[derive(Default, Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalConfig {
	#[serde(default)]
	pub targets: Vec<Target>,
	#[serde(default)]
	pub policies: Vec<rbac::Rule>,
	#[serde(default)]
	pub listener: Listener,
}


impl LocalClient {
	#[instrument(skip_all, name = "local_client")]
	pub async fn run(self) -> Result<(), anyhow::Error> {
		debug!(
			"load local config: {}",
			serde_yaml::to_string(&self.cfg).unwrap_or_default()
		);
		let mut state = self.state.write().unwrap();
		// Clear the state
		state.targets.clear();
		state.policies.clear();
		let num_targets = self.cfg.targets.len();
		let num_policies = self.cfg.policies.len();
		for target in self.cfg.targets {
			trace!("inserting target {}", &target.name);
			state.targets.insert(target).await;
		}
		let rule_set = rbac::RuleSet::new("test".to_string(), "test".to_string(), self.cfg.policies);
		state.policies.insert(rule_set);
		info!(%num_targets, %num_policies, "local config initialized");
		Ok(())
	}
}
