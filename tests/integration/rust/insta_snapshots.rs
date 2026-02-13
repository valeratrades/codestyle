use crate::utils::{assert_check_passing, opts_for, test_case, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("insta_inline_snapshot")
}

// === Passing cases (insta-inline-snapshot) ===

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
fn single_snapshot_in_function_passes() {
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
fn snapshots_in_different_functions_passes() {
	assert_check_passing(
		r#"
		fn test_a() {
			insta::assert_snapshot!(a, @"");
		}
		fn test_b() {
			insta::assert_snapshot!(b, @"");
		}
		"#,
		&opts(),
	);
}

// === Violation cases (insta-inline-snapshot with autofix) ===

#[test]
fn snapshot_without_inline() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[insta-inline-snapshot] /main.rs:3: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`

	# Format mode
	fn test() {
		let output = "hello";
		insta::assert_snapshot!(output, @"");
	}
	"#);
}

#[test]
fn assert_debug_snapshot_without_inline() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let output = vec![1, 2, 3];
			insta::assert_debug_snapshot!(output);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[insta-inline-snapshot] /main.rs:3: `assert_debug_snapshot!` must use inline snapshot with `@r""` or `@""`

	# Format mode
	fn test() {
		let output = vec![1, 2, 3];
		insta::assert_debug_snapshot!(output, @"");
	}
	"#);
}

#[test]
fn assert_json_snapshot_without_inline() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let output = serde_json::json!({"key": "value"});
			insta::assert_json_snapshot!(output);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[insta-inline-snapshot] /main.rs:3: `assert_json_snapshot!` must use inline snapshot with `@r""` or `@""`

	# Format mode
	fn test() {
		let output = serde_json::json!({"key": "value"});
		insta::assert_json_snapshot!(output, @"");
	}
	"#);
}

#[test]
fn snapshot_in_tests_directory() {
	insta::assert_snapshot!(test_case(
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
	), @r#"
	# Assert mode
	[insta-inline-snapshot] /tests/test.rs:2: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`

	# Format mode
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
fn integration_test_file_with_rstest() {
	insta::assert_snapshot!(test_case(
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
	# Assert mode
	[insta-inline-snapshot] /tests/integration/a.rs:4: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`

	# Format mode
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
	insta::assert_snapshot!(test_case(
		r#"
		//- /Cargo.toml
		[package]
		name = "test"
		version = "0.1.0"

		//- /tests/test.rs
		fn test() {
			insta::assert_snapshot!(output);
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
	# Assert mode
	[insta-inline-snapshot] /tests/test.rs:2: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`

	# Format mode
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
	insta::assert_snapshot!(test_case(
		r#"
		//- /Cargo.toml
		[package]
		name = "test"
		version = "0.1.0"

		//- /src/lib.rs
		fn foo() {}

		//- /tests/test.rs
		fn test() {
			insta::assert_snapshot!(output);
		}

		//- /tests/snapshots/test__foo.snap.pending-snap
		---
		source: tests/test.rs
		expression: result
		---
		pending content
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[insta-inline-snapshot] /tests/test.rs:2: `assert_snapshot!` must use inline snapshot with `@r""` or `@""`

	# Format mode
	//- /Cargo.toml
	[package]
	name = "test"
	version = "0.1.0"

	//- /src/lib.rs
	fn foo() {}

	//- /tests/test.rs
	fn test() {
		insta::assert_snapshot!(output, @"");
	}
	"#);
}

// === Violation cases (insta-sequential-snapshots - no autofix) ===

#[test]
fn sequential_snapshots_two_in_function() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn test() {
			insta::assert_snapshot!(a, @"");
			insta::assert_snapshot!(b, @"");
		}
		"#,
		&opts(),
	), @"[insta-sequential-snapshots] /main.rs:3: multiple snapshot assertions in one test (first at line 2); join tested strings together or split into separate tests");
}

#[test]
fn sequential_snapshots_different_variants() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn test() {
			insta::assert_snapshot!(a, @"");
			insta::assert_debug_snapshot!(b, @"");
		}
		"#,
		&opts(),
	), @"[insta-sequential-snapshots] /main.rs:3: multiple snapshot assertions in one test (first at line 2); join tested strings together or split into separate tests");
}

#[test]
fn sequential_snapshots_three_in_function() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn test() {
			insta::assert_snapshot!(a, @"");
			insta::assert_snapshot!(b, @"");
			insta::assert_snapshot!(c, @"");
		}
		"#,
		&opts(),
	), @"[insta-sequential-snapshots] /main.rs:3: multiple snapshot assertions in one test (first at line 2); join tested strings together or split into separate tests");
}

#[test]
fn sequential_snapshots_separated_by_statement() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn test() {
			insta::assert_snapshot!(a, @"");
			let x = compute();
			insta::assert_snapshot!(b, @"");
		}
		"#,
		&opts(),
	), @"[insta-sequential-snapshots] /main.rs:4: multiple snapshot assertions in one test (first at line 2); join tested strings together or split into separate tests");
}

// === Mixed: both inline-snapshot and sequential violations ===

#[test]
fn multiple_violations_inline_and_sequential() {
	insta::assert_snapshot!(test_case_assert_only(
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
	[insta-sequential-snapshots] /main.rs:3: multiple snapshot assertions in one test (first at line 2); join tested strings together or split into separate tests
	"#);
}

// === Cross-group: snapshot + assert is fine ===

#[test]
fn snapshot_and_assert_eq_passes() {
	assert_check_passing(
		r#"
		fn test() {
			insta::assert_snapshot!(a, @"");
			assert_eq!(x, y);
		}
		"#,
		&opts(),
	);
}

#[test]
fn snapshot_and_assert_passes() {
	assert_check_passing(
		r#"
		fn test() {
			assert!(condition);
			insta::assert_debug_snapshot!(a, @"");
		}
		"#,
		&opts(),
	);
}

// === Non-snapshot assert group violations ===

#[test]
fn sequential_asserts_two_in_function() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn test() {
			assert_eq!(a, b);
			assert_eq!(c, d);
		}
		"#,
		&opts(),
	), @"[insta-sequential-snapshots] /main.rs:3: multiple assert macros in one test (first at line 2); split into separate tests");
}

#[test]
fn sequential_asserts_mixed_variants() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn test() {
			assert!(condition);
			assert_ne!(a, b);
		}
		"#,
		&opts(),
	), @"[insta-sequential-snapshots] /main.rs:3: multiple assert macros in one test (first at line 2); split into separate tests");
}

#[test]
fn sequential_asserts_three_warns_once() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn test() {
			assert!(a);
			assert_eq!(b, c);
			assert_ne!(d, e);
		}
		"#,
		&opts(),
	), @"[insta-sequential-snapshots] /main.rs:3: multiple assert macros in one test (first at line 2); split into separate tests");
}
