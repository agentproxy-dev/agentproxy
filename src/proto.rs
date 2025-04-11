use std::collections::HashMap;

pub mod aidp {
	pub mod dev {

		#[allow(clippy::all)]
		pub mod common {
			tonic::include_proto!("aidp.dev.common");
			include!(concat!(env!("OUT_DIR"), "/aidp.dev.common.serde.rs"));
		}
		pub mod mcp {
			#[allow(clippy::all)]
			pub mod rbac {
				tonic::include_proto!("aidp.dev.mcp.rbac");
				include!(concat!(env!("OUT_DIR"), "/aidp.dev.mcp.rbac.serde.rs"));
			}
			#[allow(clippy::all)]
			pub mod target {
				tonic::include_proto!("aidp.dev.mcp.target");
				include!(concat!(env!("OUT_DIR"), "/aidp.dev.mcp.target.serde.rs"));
			}
			#[allow(clippy::all)]
			pub mod listener {
				tonic::include_proto!("aidp.dev.mcp.listener");
				include!(concat!(env!("OUT_DIR"), "/aidp.dev.mcp.listener.serde.rs"));
			}
		}
		pub mod a2a {
			#[allow(clippy::all)]
			pub mod listener {
				tonic::include_proto!("aidp.dev.a2a.listener");
				include!(concat!(env!("OUT_DIR"), "/aidp.dev.a2a.listener.serde.rs"));
			}
			#[allow(clippy::all)]
			pub mod target {
				tonic::include_proto!("aidp.dev.a2a.target");
				include!(concat!(env!("OUT_DIR"), "/aidp.dev.a2a.target.serde.rs"));
			}
			#[allow(clippy::all)]
			pub mod rbac {
				tonic::include_proto!("aidp.dev.a2a.rbac");
				include!(concat!(env!("OUT_DIR"), "/aidp.dev.a2a.rbac.serde.rs"));
			}
		}
	}
}

pub fn resolve_local_data_source(
	local_data_source: &aidp::dev::common::local_data_source::Source,
) -> Result<Vec<u8>, std::io::Error> {
	match local_data_source {
		aidp::dev::common::local_data_source::Source::FilePath(file_path) => {
			let file = std::fs::read(file_path)?;
			Ok(file)
		},
		aidp::dev::common::local_data_source::Source::Inline(inline) => Ok(inline.clone()),
	}
}

pub fn resolve_header_map(
	headers: &[aidp::dev::common::Header],
) -> Result<HashMap<String, String>, std::io::Error> {
	let mut header_map = HashMap::new();
	for header in headers {
		match &header.value {
			Some(aidp::dev::common::header::Value::StringValue(value)) => {
				header_map.insert(header.key.clone(), value.clone());
			},
			Some(aidp::dev::common::header::Value::EnvValue(value)) => {
				header_map.insert(
					header.key.clone(),
					std::env::var(value).map_err(|_| {
						std::io::Error::new(
							std::io::ErrorKind::InvalidData,
							"Failed to resolve environment variable",
						)
					})?,
				);
			},
			_ => {
				return Err(std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					"Unsupported header value type",
				));
			},
		}
	}
	Ok(header_map)
}
