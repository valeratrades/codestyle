use std::process::Command;

use v_fixtures::Fixture;

fn report(fixture_str: &str) -> String {
	let temp = Fixture::parse(fixture_str).write_to_tempdir();
	let out = Command::new(env!("CARGO_BIN_EXE_codestyle"))
		.args(["rust", "--only", "swappable-args", "--only", "loops", "assert", "."])
		.current_dir(&temp.root)
		.output()
		.expect("binary is built by cargo for integration tests");
	assert_eq!(out.status.code(), Some(1), "violations must fail the run");
	let stderr = String::from_utf8(out.stderr).expect("report is utf-8");
	stderr[stderr.find("codestyle:").expect("report header is printed")..].to_string() // drops the v_utils tracing preamble
}

#[test]
fn workspace_groups_by_rule_then_crate() {
	insta::assert_snapshot!(report(
		r#"
		//- /Cargo.toml
		[workspace]
		members = ["a", "b"]

		//- /a/Cargo.toml
		[package]
		name = "a"

		//- /a/src/lib.rs
		fn spin() {
			loop {}
		}
		fn pair(x: &str, y: &str) {}

		//- /b/Cargo.toml
		[package]
		name = "b"

		//- /b/src/lib.rs
		fn spin() {
			loop {}
		}
		fn other() {
			loop {}
		}
	"#,
	), @"
	codestyle: found 4 violation(s):

	- [loop-comment]
		HINT: try to rewrite the loop with `while let` or justify why a bound can't be enforced
		- a/
			- ./a/src/lib.rs:2:1: Endless loop without `//LOOP` comment
		- b/
			- ./b/src/lib.rs:2:1: Endless loop without `//LOOP` comment
			- ./b/src/lib.rs:5:1: Endless loop without `//LOOP` comment
	- [swappable-args]
		HINT: wrap them in newtypes, or pass an args struct
		- a/
			- ./a/src/lib.rs:4:3: `pair` takes interchangeable parameters (2 x `&str`)
	");
}

#[test]
fn single_crate_drops_crate_level() {
	insta::assert_snapshot!(report(
		r#"
		//- /Cargo.toml
		[package]
		name = "solo"

		//- /src/lib.rs
		fn spin() {
			loop {}
		}
		fn pair(x: &str, y: &str) {}
	"#,
	), @"
	codestyle: found 2 violation(s):

	- [loop-comment]
		HINT: try to rewrite the loop with `while let` or justify why a bound can't be enforced
		- ./src/lib.rs:2:1: Endless loop without `//LOOP` comment
	- [swappable-args]
		HINT: wrap them in newtypes, or pass an args struct
		- ./src/lib.rs:4:3: `pair` takes interchangeable parameters (2 x `&str`)
	");
}
