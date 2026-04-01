use codestyle::rust_checks::workspace_dep_hoisting;
use v_fixtures::Fixture;

fn check_fixture(fixture_str: &str) -> Vec<String> {
	let fixture = Fixture::parse(fixture_str);
	let temp = fixture.write_to_tempdir();
	let violations = workspace_dep_hoisting::check(&temp.root);
	violations
		.iter()
		.map(|v| {
			let relative_path = v.file.strip_prefix(temp.root.to_str().unwrap_or("")).unwrap_or(&v.file);
			let relative_path = relative_path.trim_start_matches('/');
			format!("[{}] /{relative_path}:{}: {}", v.rule, v.line, v.message)
		})
		.collect()
}

// === Passing cases ===

#[test]
fn standalone_crate_no_workspace_passes() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[package]
		name = "my-crate"

		[dependencies]
		tokio = "1"
		serde = "1"
	"#,
	);
	assert!(violations.is_empty(), "expected no violations, got: {violations:#?}");
}

#[test]
fn shared_dep_already_in_workspace_deps_passes() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["crate-a", "crate-b"]

		[workspace.dependencies]
		tokio = "1"

		//- /crate-a/Cargo.toml
		[package]
		name = "crate-a"

		[dependencies]
		tokio.workspace = true

		//- /crate-b/Cargo.toml
		[package]
		name = "crate-b"

		[dependencies]
		tokio.workspace = true
	"#,
	);
	assert!(violations.is_empty(), "expected no violations, got: {violations:#?}");
}

#[test]
fn dep_used_by_only_one_member_passes() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["crate-a", "crate-b"]

		//- /crate-a/Cargo.toml
		[package]
		name = "crate-a"

		[dependencies]
		tokio = "1"

		//- /crate-b/Cargo.toml
		[package]
		name = "crate-b"

		[dependencies]
		serde = "1"
	"#,
	);
	assert!(violations.is_empty(), "expected no violations, got: {violations:#?}");
}

#[test]
fn path_deps_not_flagged() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["crate-a", "crate-b"]

		//- /crate-a/Cargo.toml
		[package]
		name = "crate-a"

		[dependencies]
		my-lib = { path = "../my-lib" }

		//- /crate-b/Cargo.toml
		[package]
		name = "crate-b"

		[dependencies]
		my-lib = { path = "../my-lib" }
	"#,
	);
	assert!(violations.is_empty(), "expected no violations, got: {violations:#?}");
}

// === Violation cases ===

#[test]
fn shared_dep_not_hoisted_is_flagged() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["crate-a", "crate-b"]

		//- /crate-a/Cargo.toml
		[package]
		name = "crate-a"

		[dependencies]
		tokio = "1"

		//- /crate-b/Cargo.toml
		[package]
		name = "crate-b"

		[dependencies]
		tokio = "1"
	"#,
	);
	insta::assert_snapshot!(violations.join("\n"), @"[workspace-dep-hoisting] /Cargo.toml:1: dependency `tokio` is used by 2 members (crate-a, crate-b) but not declared in [workspace.dependencies]");
}

#[test]
fn multiple_shared_deps_all_flagged() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["crate-a", "crate-b"]

		//- /crate-a/Cargo.toml
		[package]
		name = "crate-a"

		[dependencies]
		serde = "1"
		tokio = "1"

		//- /crate-b/Cargo.toml
		[package]
		name = "crate-b"

		[dependencies]
		serde = "1"
		tokio = "1"
	"#,
	);
	insta::assert_snapshot!(violations.join("\n"), @r"
	[workspace-dep-hoisting] /Cargo.toml:1: dependency `serde` is used by 2 members (crate-a, crate-b) but not declared in [workspace.dependencies]
	[workspace-dep-hoisting] /Cargo.toml:1: dependency `tokio` is used by 2 members (crate-a, crate-b) but not declared in [workspace.dependencies]
	");
}

#[test]
fn partial_hoisting_only_flags_missing() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["crate-a", "crate-b"]

		[workspace.dependencies]
		serde = "1"

		//- /crate-a/Cargo.toml
		[package]
		name = "crate-a"

		[dependencies]
		serde.workspace = true
		tokio = "1"

		//- /crate-b/Cargo.toml
		[package]
		name = "crate-b"

		[dependencies]
		serde.workspace = true
		tokio = "1"
	"#,
	);
	insta::assert_snapshot!(violations.join("\n"), @"[workspace-dep-hoisting] /Cargo.toml:1: dependency `tokio` is used by 2 members (crate-a, crate-b) but not declared in [workspace.dependencies]");
}

#[test]
fn dev_dependencies_also_checked() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["crate-a", "crate-b"]

		//- /crate-a/Cargo.toml
		[package]
		name = "crate-a"

		[dev-dependencies]
		insta = "1"

		//- /crate-b/Cargo.toml
		[package]
		name = "crate-b"

		[dev-dependencies]
		insta = "1"
	"#,
	);
	insta::assert_snapshot!(violations.join("\n"), @"[workspace-dep-hoisting] /Cargo.toml:1: dependency `insta` is used by 2 members (crate-a, crate-b) but not declared in [workspace.dependencies]");
}

#[test]
fn three_member_shared_dep_shows_all() {
	let violations = check_fixture(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["crate-a", "crate-b", "crate-c"]

		//- /crate-a/Cargo.toml
		[package]
		name = "crate-a"

		[dependencies]
		tokio = "1"

		//- /crate-b/Cargo.toml
		[package]
		name = "crate-b"

		[dependencies]
		tokio = "1"

		//- /crate-c/Cargo.toml
		[package]
		name = "crate-c"

		[dependencies]
		tokio = "1"
	"#,
	);
	insta::assert_snapshot!(violations.join("\n"), @"[workspace-dep-hoisting] /Cargo.toml:1: dependency `tokio` is used by 3 members (crate-a, crate-b, crate-c) but not declared in [workspace.dependencies]");
}
