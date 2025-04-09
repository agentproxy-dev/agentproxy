pub mod mcpproxy {
	pub mod dev {
		#[allow(clippy::all)]
		pub mod rbac {
			tonic::include_proto!("mcpproxy.dev.rbac");
			include!(concat!(env!("OUT_DIR"), "/mcpproxy.dev.rbac.serde.rs"));
		}
		#[allow(clippy::all)]
		pub mod target {
			tonic::include_proto!("mcpproxy.dev.target");
			include!(concat!(env!("OUT_DIR"), "/mcpproxy.dev.target.serde.rs"));
		}
		#[allow(clippy::all)]
		pub mod listener {
			tonic::include_proto!("mcpproxy.dev.listener");
			include!(concat!(env!("OUT_DIR"), "/mcpproxy.dev.listener.serde.rs"));
		}

		#[allow(clippy::all)]
		pub mod common {
			tonic::include_proto!("mcpproxy.dev.common");
			include!(concat!(env!("OUT_DIR"), "/mcpproxy.dev.common.serde.rs"));
		}
	}
}

pub fn resolve_local_data_source(
	local_data_source: &mcpproxy::dev::common::local_data_source::Source,
) -> Result<Vec<u8>, std::io::Error> {
	match local_data_source {
		mcpproxy::dev::common::local_data_source::Source::FilePath(file_path) => {
			let file = std::fs::read(file_path)?;
			Ok(file)
		},
		mcpproxy::dev::common::local_data_source::Source::Inline(inline) => Ok(inline.clone()),
	}
}
