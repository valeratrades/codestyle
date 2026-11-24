use std::process::Command;

use codestyle::rust_checks::no_c_tls;
use v_fixtures::Fixture;

use crate::utils::render_violation;

/// Fake `native-tls` / `reqwest` under `/vendor`, wired like the real feature tables.
const VENDOR: &str = r#"
//- /vendor/native-tls/Cargo.toml
[package]
name = "native-tls"
version = "0.2.0"
edition = "2024"
//- /vendor/native-tls/src/lib.rs
//- /vendor/reqwest/Cargo.toml
[package]
name = "reqwest"
version = "0.13.0"
edition = "2024"
[dependencies]
native-tls = { path = "../native-tls", optional = true }
[features]
native-tls = ["dep:native-tls"]
rustls-no-provider = []
//- /vendor/reqwest/src/lib.rs
"#;

fn check(fixture_str: &str) -> String {
	let vendor: String = VENDOR.lines().map(|l| format!("\t\t{l}\n")).collect();
	let temp = Fixture::parse(&format!("{}\n{vendor}", fixture_str.trim_end())).write_to_tempdir();
	let status = Command::new("cargo").args(["generate-lockfile", "--offline"]).current_dir(&temp.root).status().unwrap();
	assert!(status.success());
	no_c_tls::openssl(&temp.root).iter().map(|v| render_violation(v, &temp.root)).collect::<Vec<_>>().join("\n")
}

#[test]
fn rustls_only_graph_passes() {
	insta::assert_snapshot!(check(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["app"]
		exclude = ["vendor"]
		//- /app/Cargo.toml
		[package]
		name = "app"
		version = "0.1.0"
		edition = "2024"
		[dependencies]
		reqwest = { path = "../vendor/reqwest", features = ["rustls-no-provider"] }
		//- /app/src/lib.rs
	"#,
	), @"");
}

#[test]
fn native_tls_feature_is_flagged() {
	insta::assert_snapshot!(check(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["app"]
		exclude = ["vendor"]
		//- /app/Cargo.toml
		[package]
		name = "app"
		version = "0.1.0"
		edition = "2024"
		[dependencies]
		reqwest = { path = "../vendor/reqwest", features = ["native-tls"] }
		//- /app/src/lib.rs
	"#,
	), @"
	[no-openssl] /vendor/reqwest/Cargo.toml:6: `reqwest` enables `native-tls/default`
	HINT: links system OpenSSL; set `default-features = false` and use `rustls-no-provider` (plus `rustls::crypto::ring::default_provider().install_default()` before every client build)
	");
}
