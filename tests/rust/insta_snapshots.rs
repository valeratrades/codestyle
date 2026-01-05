use crate::utils::{assert_check_passing, opts_for, simulate_check, simulate_format};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("insta_inline_snapshot")
}

#[test]
fn snapshot_without_inline_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output);
		}
		"#,
		&opts(),
	), @r#"[insta-inline-snapshot] /main.rs:3: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`"#);
}

#[test]
fn snapshot_with_inline_passes() {
	assert_check_passing(
		r#"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output, @"hello");
		}
		"#,
		&opts(),
	);
}

#[test]
fn snapshot_with_empty_inline_passes() {
	assert_check_passing(
		r#"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output, @"");
		}
		"#,
		&opts(),
	);
}

#[test]
fn raw_string_inline_passes() {
	assert_check_passing(
		r##"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output, @r#"hello"#);
		}
		"##,
		&opts(),
	);
}

#[test]
fn assert_debug_snapshot_variant() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let output = vec![1, 2, 3];
			insta::assert_debug_snapshot!(output);
		}
		"#,
		&opts(),
	), @r#"[insta-inline-snapshot] /main.rs:3: `assert_debug_snapshot!` must use inline snapshot with `@r""` or `@""`"#);
}

#[test]
fn assert_json_snapshot_variant() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let output = serde_json::json!({"key": "value"});
			insta::assert_json_snapshot!(output);
		}
		"#,
		&opts(),
	), @r#"[insta-inline-snapshot] /main.rs:3: `assert_json_snapshot!` must use inline snapshot with `@r""` or `@""`"#);
}

#[test]
fn multiline_snapshot_with_content_passes() {
	assert_check_passing(
		r#"
		fn test() {
			assert_snapshot!(extract_blockers_section(content).unwrap(), @"
				# Phase 1
				- First task
				");
		}
		"#,
		&opts(),
	);
}

#[test]
fn single_line_non_empty_snapshot_passes() {
	assert_check_passing(
		r#"
		fn test() {
			assert_snapshot!(get_current_blocker_from_content(blockers_content).unwrap(), @"- Third task");
		}
		"#,
		&opts(),
	);
}

#[test]
fn raw_string_snapshot_with_content_passes() {
	assert_check_passing(
		r##"
		fn test() {
			assert_snapshot!(format!("{:?}", items), @r#"[("Phase 1", true, false)]"#);
		}
		"##,
		&opts(),
	);
}

#[test]
fn multiple_snapshots_in_one_file() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			insta::assert_snapshot!("a");
			insta::assert_snapshot!("b", @"");
			insta::assert_debug_snapshot!(vec![1]);
		}
		"#,
		&opts(),
	), @r#"
	[insta-inline-snapshot] /main.rs:2: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`
	[insta-inline-snapshot] /main.rs:4: `assert_debug_snapshot!` must use inline snapshot with `@r""` or `@""`
	"#);
}

#[test]
fn run_assert_scans_tests_directory() {
	insta::assert_snapshot!(simulate_check(
		r#"
		//- /Cargo.toml
		[package]
		name = "test"
		version = "0.1.0"

		//- /tests/test.rs
		fn test() {
			insta::assert_snapshot!(output);
		}
		"#,
		&opts(),
	), @r#"[insta-inline-snapshot] /tests/test.rs:2: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`"#);
}

#[test]
fn integration_test_file_with_rstest_detected() {
	insta::assert_snapshot!(simulate_check(
		r#"
		//- /Cargo.toml
		[package]
		name = "test"
		version = "0.1.0"

		//- /tests/integration/a.rs
		#[rstest]
		fn test_with_invalid_snapshot_usage_pattern() {
			let s = "123";
			insta::assert_snapshot!(s);
		}
		"#,
		&opts(),
	), @r#"[insta-inline-snapshot] /tests/integration/a.rs:4: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`"#);
}

#[test]
fn integration_test_file_autofix() {
	insta::assert_snapshot!(simulate_format(
		r#"
		//- /Cargo.toml
		[package]
		name = "test"
		version = "0.1.0"

		//- /tests/integration/a.rs
		#[rstest]
		fn test_with_invalid_snapshot_usage_pattern() {
			let s = "123";
			insta::assert_snapshot!(s);
		}
		"#,
		&opts(),
	), @r#"
	//- /Cargo.toml
	[package]
	name = "test"
	version = "0.1.0"

	//- /tests/integration/a.rs
	#[rstest]
	fn test_with_invalid_snapshot_usage_pattern() {
		let s = "123";
		insta::assert_snapshot!(s, @"");
	}
	"#);
}

#[test]
fn format_deletes_snap_files() {
	// Verifies that .snap files are deleted during format when insta check is enabled
	insta::assert_snapshot!(simulate_format(
		r#"
		//- /Cargo.toml
		[package]
		name = "test"
		version = "0.1.0"

		//- /tests/test.rs
		fn test() {
			insta::assert_snapshot!(output, @"");
		}

		//- /tests/snapshots/test__some_test.snap
		---
		source: tests/test.rs
		expression: output
		---
		hello
		"#,
		&opts(),
	), @r#"
	//- /Cargo.toml
	[package]
	name = "test"
	version = "0.1.0"

	//- /tests/test.rs
	fn test() {
		insta::assert_snapshot!(output, @"");
	}
	"#);
}

#[test]
fn format_deletes_pending_snap_files() {
	// Verifies that .pending-snap files are also deleted
	insta::assert_snapshot!(simulate_format(
		r#"
		//- /Cargo.toml
		[package]
		name = "test"
		version = "0.1.0"

		//- /src/lib.rs
		fn foo() {}

		//- /tests/snapshots/test__foo.snap.pending-snap
		---
		source: tests/test.rs
		expression: result
		---
		pending content
		"#,
		&opts(),
	), @r#"
	//- /Cargo.toml
	[package]
	name = "test"
	version = "0.1.0"

	//- /src/lib.rs
	fn foo() {}
	"#);
}
