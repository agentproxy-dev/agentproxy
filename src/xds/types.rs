use crate::strng;
use crate::strng::Strng;

// We don't control the codegen, so disable any code warnings in the
// proto modules.
#[allow(warnings)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub mod envoy {
	pub mod service {
		pub mod discovery {
			pub mod v3 {
				tonic::include_proto!("envoy.service.discovery.v3");
			}
		}
	}
}

pub mod mcp {
	pub mod kgateway_dev {
		pub mod rbac {
			tonic::include_proto!("mcp.kgateway.dev.rbac.v1alpha1");
		}
		#[allow(clippy::all)]
		pub mod target {
			tonic::include_proto!("mcp.kgateway.dev.target.v1alpha1");
			include!(concat!(
				env!("OUT_DIR"),
				"/mcp.kgateway.dev.target.v1alpha1.serde.rs"
			));
		}
		#[allow(clippy::all)]
		pub mod listener {
			tonic::include_proto!("mcp.kgateway.dev.listener.v1alpha1");
			include!(concat!(
				env!("OUT_DIR"),
				"/mcp.kgateway.dev.listener.v1alpha1.serde.rs"
			));
		}
	}
}

pub const TARGET_TYPE: Strng =
	strng::literal!("type.googleapis.com/mcp.kgateway.dev.target.v1alpha1.Target");
pub const RBAC_TYPE: Strng =
	strng::literal!("type.googleapis.com/mcp.kgateway.dev.rbac.v1alpha1.Config");
