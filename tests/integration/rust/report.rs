use std::process::Command;

use v_fixtures::Fixture;

fn report(fixture_str: &str) -> String {
	let temp = Fixture::parse(fixture_str).write_to_tempdir();
	let out = Command::new(env!("CARGO_BIN_EXE_codestyle"))
		.args(["rust", "--only", "swappable-args", "--only", "loops", "--only", "insta-inline-snapshot", "assert", "."])
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
	  HINT: endless loop without `//LOOP` comment; try to rewrite the loop with `while let` or justify why a bound can't be enforced
	  - a/
	    - ./a/src/lib.rs:2:1
	  - b/
	    - ./b/src/lib.rs:2:1
	    - ./b/src/lib.rs:5:1
	- [swappable-args]
	  HINT: fn takes interchangeable parameters; wrap them in newtypes, or pass an args struct
	  - a/
	    - ./a/src/lib.rs:4:3: `pair` (2 x `&str`)
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
	  HINT: endless loop without `//LOOP` comment; try to rewrite the loop with `while let` or justify why a bound can't be enforced
	  - ./src/lib.rs:2:1
	- [swappable-args]
	  HINT: fn takes interchangeable parameters; wrap them in newtypes, or pass an args struct
	  - ./src/lib.rs:4:3: `pair` (2 x `&str`)
	");
}

#[test]
fn general_hint_is_printed_once_above_its_locations() {
	insta::assert_snapshot!(report(
		r#"
		//- /Cargo.toml
		[package]
		name = "solo"

		//- /src/lib.rs
		fn one() {
			insta::assert_snapshot!(a, @"");
			insta::assert_snapshot!(b, @"");
		}
		fn two() {
			insta::assert_snapshot!(a, @"");
			insta::assert_snapshot!(b, @"");
		}
	"#,
	), @"
	codestyle: found 2 violation(s):

	- [insta-sequential-snapshots]
	  HINT: multiple snapshot assertions in one test; join tested strings together or split into separate tests
	  - ./src/lib.rs:3:1: first at line 2
	  - ./src/lib.rs:7:1: first at line 6
	");
}
