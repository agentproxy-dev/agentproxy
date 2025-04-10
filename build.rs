use std::env;
use std::process::Command;

// This build script is used to generate the rust source files that
// we need for XDS GRPC communication.
fn main() -> Result<(), anyhow::Error> {
	// Fuzzing uses custom cfg (https://rust-fuzz.github.io/book/cargo-fuzz/guide.html)
	// Tell cargo to expect this (https://doc.rust-lang.org/nightly/rustc/check-cfg/cargo-specifics.html).
	println!("cargo::rustc-check-cfg=cfg(fuzzing)");
	let proto_files = [
		"proto/a2a/listener.proto",
		"proto/a2a/target.proto",
		"proto/a2a/rbac.proto",
		"proto/mcp/listener.proto",
		"proto/mcp/target.proto",
		"proto/mcp/rbac.proto",
		"proto/xds.proto",
		"proto/common.proto",
	]
	.iter()
	.map(|name| std::env::current_dir().unwrap().join(name))
	.collect::<Vec<_>>();
	let include_dirs = ["proto/"]
		.iter()
		.map(|i| std::env::current_dir().unwrap().join(i))
		.collect::<Vec<_>>();
	let config = {
		let mut c = prost_build::Config::new();
		c.disable_comments(Some("."));
		c
	};

	let out_dir = env::var("OUT_DIR").unwrap();
	let descriptor_path = std::path::PathBuf::from(out_dir.clone()).join("proto_descriptor.bin");

	tonic_build::configure()
		.build_server(true)
		.file_descriptor_set_path(descriptor_path.clone())
		.compile_well_known_types(true)
		.extern_path(".google.protobuf", "::pbjson_types")
		.compile_protos_with_config(
			config,
			&proto_files
				.iter()
				.map(|path| path.to_str().unwrap())
				.collect::<Vec<_>>(),
			&include_dirs
				.iter()
				.map(|p| p.to_str().unwrap())
				.collect::<Vec<_>>(),
		)?;

	// Adoppted from https://github.com/uutils/coreutils/blob/main/src/uu/stdbuf/build.rs
	let profile_name = out_dir
		.split(std::path::MAIN_SEPARATOR)
		.nth_back(3)
		.unwrap();

	match Command::new("common/scripts/report_build_info.sh").output() {
		Ok(output) => {
			for line in String::from_utf8(output.stdout).unwrap().lines() {
				// Each line looks like `mcp-proxy.dev.buildGitRevision=abc`
				if let Some((key, value)) = line.split_once('=') {
					#[allow(clippy::double_ended_iterator_last)]
					let key = key.split('.').last().unwrap();
					println!("cargo:rustc-env=MCPGW_BUILD_{key}={value}");
				} else {
					println!("cargo:warning=invalid build output {line}");
				}
			}
		},
		Err(err) => {
			println!("cargo:warning={err}");
		},
	};
	println!(
		"cargo:rustc-env=MCPGW_BUILD_RUSTC_VERSION={}",
		rustc_version::version().unwrap()
	);
	println!("cargo:rustc-env=MCPGW_BUILD_PROFILE_NAME={}", profile_name);

	// This tells cargo to re-run this build script only when the proto files
	// we're interested in change or the any of the proto directories were updated.
	for path in [proto_files, include_dirs].concat() {
		println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
	}
	let descriptor_set = std::fs::read(descriptor_path).expect("descriptors not present");
	pbjson_build::Builder::new()
		.register_descriptors(&descriptor_set)?
		.build(&[
			".aidp.dev.a2a.target",
			".aidp.dev.a2a.rbac",
			".aidp.dev.a2a.listener",
			".aidp.dev.mcp.target",
			".aidp.dev.mcp.rbac",
			".aidp.dev.mcp.listener",
			".aidp.dev.common",
		])?;

	Ok(())
}
