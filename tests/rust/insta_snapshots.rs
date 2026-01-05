use codestyle::{
	rust_checks::{self, RustCheckOptions, Violation, insta_snapshots, run_assert},
	test_fixture::Fixture,
};

/// Check that given code produces expected violations
fn check_violations(code: &str, expected: &[&str]) {
	let fixture = Fixture::parse(code);
	let temp = fixture.write_to_tempdir();

	let file_infos = rust_checks::collect_rust_files(&temp.root);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| insta_snapshots::check(&info.path, &info.contents, tree, false))
		.collect();

	let messages: Vec<&str> = violations.iter().map(|v| v.message.as_str()).collect();

	assert_eq!(messages, expected, "Violations mismatch for fixture:\n{code}");
}

/// Check that given code produces no violations
fn check_ok(code: &str) {
	check_violations(code, &[]);
}

fn main() {
	// === Detection tests ===

	// Snapshot without inline @"" is violation
	check_violations(
		r#"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output);
		}
		"#,
		&[r#"`assert_snapshot!` must use inline snapshot with `@r""` or `@""`"#],
	);

	// Snapshot with inline passes
	check_ok(
		r#"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output, @"hello");
		}
		"#,
	);

	// Snapshot with empty inline passes
	check_ok(
		r#"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output, @"");
		}
		"#,
	);

	// Raw string inline passes
	check_ok(
		r##"
		fn test() {
			let output = "hello";
			insta::assert_snapshot!(output, @r#"hello"#);
		}
		"##,
	);

	// assert_debug_snapshot variant
	check_violations(
		r#"
		fn test() {
			let output = vec![1, 2, 3];
			insta::assert_debug_snapshot!(output);
		}
		"#,
		&[r#"`assert_debug_snapshot!` must use inline snapshot with `@r""` or `@""`"#],
	);

	// assert_json_snapshot variant
	check_violations(
		r#"
		fn test() {
			let output = serde_json::json!({"key": "value"});
			insta::assert_json_snapshot!(output);
		}
		"#,
		&[r#"`assert_json_snapshot!` must use inline snapshot with `@r""` or `@""`"#],
	);

	// Multiline snapshot with content passes
	check_ok(
		r#"
		fn test() {
			assert_snapshot!(extract_blockers_section(content).unwrap(), @"
				# Phase 1
				- First task
				");
		}
		"#,
	);

	// Single-line non-empty snapshot passes
	check_ok(
		r#"
		fn test() {
			assert_snapshot!(get_current_blocker_from_content(blockers_content).unwrap(), @"- Third task");
		}
		"#,
	);

	// Raw string snapshot with content passes
	check_ok(
		r##"
		fn test() {
			assert_snapshot!(format!("{:?}", items), @r#"[("Phase 1", true, false)]"#);
		}
		"##,
	);

	// Multiple snapshots in one file
	check_violations(
		r#"
		fn test() {
			insta::assert_snapshot!("a");
			insta::assert_snapshot!("b", @"");
			insta::assert_debug_snapshot!(vec![1]);
		}
		"#,
		&[
			r#"`assert_snapshot!` must use inline snapshot with `@r""` or `@""`"#,
			r#"`assert_debug_snapshot!` must use inline snapshot with `@r""` or `@""`"#,
		],
	);

	// === Directory scanning tests ===

	// run_assert should scan tests/ directory (not just src/)
	{
		let fixture = Fixture::parse(
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
		);
		let temp = fixture.write_to_tempdir();

		let opts = RustCheckOptions {
			insta_inline_snapshot: true,
			..Default::default()
		};
		let exit_code = run_assert(&temp.root, &opts);
		assert_eq!(exit_code, 1, "Should detect violations in tests/ directory");
	}

	println!("All insta_snapshots tests passed!");
}
