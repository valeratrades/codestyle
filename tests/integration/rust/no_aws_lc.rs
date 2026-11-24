use std::process::Command;

use codestyle::rust_checks::no_aws_lc;
use v_fixtures::Fixture;

use crate::utils::render_violation;

/// Fake `aws-lc-rs` / `rustls` / `tokio-rustls` / `tonic` under `/vendor`, wired like the real feature tables.
const VENDOR: &str = r#"
//- /vendor/aws-lc-rs/Cargo.toml
[package]
name = "aws-lc-rs"
version = "1.0.0"
edition = "2024"
[features]
aws-lc-sys = []
//- /vendor/aws-lc-rs/src/lib.rs
//- /vendor/rustls/Cargo.toml
[package]
name = "rustls"
version = "0.23.0"
edition = "2024"
[dependencies]
aws-lc-rs = { path = "../aws-lc-rs", optional = true, default-features = false }
[features]
aws_lc_rs = ["dep:aws-lc-rs", "aws-lc-rs/aws-lc-sys"]
ring = []
std = []
//- /vendor/rustls/src/lib.rs
//- /vendor/tokio-rustls/Cargo.toml
[package]
name = "tokio-rustls"
version = "0.26.0"
edition = "2024"
[dependencies]
rustls = { path = "../rustls", default-features = false }
[features]
aws-lc-rs = ["rustls/aws_lc_rs"]
ring = ["rustls/ring"]
//- /vendor/tokio-rustls/src/lib.rs
//- /vendor/tonic/Cargo.toml
[package]
name = "tonic"
version = "0.14.0"
edition = "2024"
[dependencies]
tokio-rustls = { path = "../tokio-rustls", optional = true }
[features]
tls-aws-lc = ["dep:tokio-rustls", "tokio-rustls/aws-lc-rs"]
tls-ring = ["dep:tokio-rustls", "tokio-rustls/ring"]
//- /vendor/tonic/src/lib.rs
"#;

fn check(fixture_str: &str) -> String {
	let vendor: String = VENDOR.lines().map(|l| format!("\t\t{l}\n")).collect();
	let temp = Fixture::parse(&format!("{}\n{vendor}", fixture_str.trim_end())).write_to_tempdir();
	let status = Command::new("cargo").args(["generate-lockfile", "--offline"]).current_dir(&temp.root).status().unwrap();
	assert!(status.success());
	no_aws_lc::check(&temp.root).iter().map(|v| render_violation(v, &temp.root)).collect::<Vec<_>>().join("\n")
}

#[test]
fn ring_only_graph_passes() {
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
		tonic = { path = "../vendor/tonic", features = ["tls-ring"] }
		//- /app/src/lib.rs
	"#,
	), @"");
}

#[test]
fn inherited_declaration_points_at_workspace_root() {
	insta::assert_snapshot!(check(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["app"]
		exclude = ["vendor"]

		[workspace.dependencies]
		tonic = { path = "vendor/tonic", features = ["tls-aws-lc"] }
		//- /app/Cargo.toml
		[package]
		name = "app"
		version = "0.1.0"
		edition = "2024"
		[dependencies]
		tonic.workspace = true
		//- /app/src/lib.rs
	"#,
	), @"
	[no-aws-lc] /Cargo.toml:6: `app` enables `tonic/tls-aws-lc`
	HINT: builds aws-lc-sys (bundled C); use `tls-ring` instead
	");
}

/// `std` reaches rustls through a package node, not through aws-lc's feature chain.
#[test]
fn crate_enabling_unrelated_rustls_feature_is_not_blamed() {
	insta::assert_snapshot!(check(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["app", "verifier"]
		exclude = ["vendor"]
		//- /app/Cargo.toml
		[package]
		name = "app"
		version = "0.1.0"
		edition = "2024"
		[dependencies]
		rustls = { path = "../vendor/rustls", features = ["aws_lc_rs"] }
		verifier = { path = "../verifier" }
		//- /app/src/lib.rs
		//- /verifier/Cargo.toml
		[package]
		name = "verifier"
		version = "0.1.0"
		edition = "2024"

		[dependencies.rustls]
		path = "../vendor/rustls"
		features = ["std"]
		//- /verifier/src/lib.rs
	"#,
	), @"
	[no-aws-lc] /app/Cargo.toml:6: `app` enables `rustls/aws_lc_rs`
	HINT: builds aws-lc-sys (bundled C); set `default-features = false`, keep the defaults you need, and add `ring` (`rustls-webpki`: `ring`); keep exactly one provider enabled, or `ClientConfig::builder()` panics on ambiguity
	");
}
