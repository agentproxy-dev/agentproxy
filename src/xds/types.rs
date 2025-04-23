use crate::strng;
use crate::strng::Strng;

// We don't control the codegen, so disable any code warnings in the
// proto modules.
#[allow(warnings)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub(crate) mod envoy {
	pub(crate) mod service {
		pub(crate) mod discovery {
			pub(crate) mod v3 {
				tonic::include_proto!("envoy.service.discovery.v3");
			}
		}
	}
}

pub const MCP_TARGET_TYPE: Strng =
	strng::literal!("type.googleapis.com/agentgateway.dev.mcp.target.Target");
pub const A2A_TARGET_TYPE: Strng =
	strng::literal!("type.googleapis.com/agentgateway.dev.a2a.target.Target");
pub const LISTENER_TYPE: Strng =
	strng::literal!("type.googleapis.com/agentgateway.dev.listener.Listener");
