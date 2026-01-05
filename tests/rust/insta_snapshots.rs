use codestyle::rust_checks::{self, RustCheckOptions, Violation, insta_snapshots, run_assert};

fn check_code(code: &str, is_format_mode: bool) -> Vec<Violation> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_insta_snapshots");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| insta_snapshots::check(&info.path, &info.contents, tree, is_format_mode))
		.collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn snapshot_violations(violations: &[Violation]) -> String {
	if violations.is_empty() {
		"(no violations)".to_string()
	} else {
		violations.iter().map(|v| &v.message).cloned().collect::<Vec<_>>().join("\n")
	}
}

fn main() {
	// Test: assert_snapshot without inline snapshot is a violation
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output);
}
"#,
		false,
	)), @r###"`assert_snapshot!` must use inline snapshot with `@r""` or `@""`"###);

	// Test: assert_snapshot with inline snapshot passes in assert mode
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @"hello");
}
"#,
		false,
	)), @"(no violations)");

	// Test: assert_snapshot with empty inline snapshot passes
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @"");
}
"#,
		false,
	)), @"(no violations)");

	// Test: assert_snapshot with raw string inline snapshot passes
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r##"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @r#"hello"#);
}
"##,
		false,
	)), @"(no violations)");

	// Test: assert_debug_snapshot variant works
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let output = vec![1, 2, 3];
    insta::assert_debug_snapshot!(output);
}
"#,
		false,
	)), @r###"`assert_debug_snapshot!` must use inline snapshot with `@r""` or `@""`"###);

	// Test: assert_json_snapshot variant works
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let output = serde_json::json!({"key": "value"});
    insta::assert_json_snapshot!(output);
}
"#,
		false,
	)), @r###"`assert_json_snapshot!` must use inline snapshot with `@r""` or `@""`"###);

	// Test: format mode should NOT touch snapshots that already have inline strings
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @"hello");
}
"#,
		true,
	)), @"(no violations)");

	// Test: format mode with empty inline snapshot passes (no change needed)
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @"");
}
"#,
		true,
	)), @"(no violations)");

	// Test: format mode should NOT touch multiline snapshots with content
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    assert_snapshot!(extract_blockers_section(content).unwrap(), @"
        # Phase 1
        - First task
        ");
}
"#,
		true,
	)), @"(no violations)");

	// Test: format mode should NOT touch single-line non-empty snapshots
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    assert_snapshot!(get_current_blocker_from_content(blockers_content).unwrap(), @"- Third task");
}
"#,
		true,
	)), @"(no violations)");

	// Test: format mode should NOT touch raw string snapshots with content
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r##"
fn test() {
    assert_snapshot!(format!("{:?}", items), @r#"[("Phase 1", true, false), ("Completed task", false, true)]"#);
}
"##,
		true,
	)), @"(no violations)");

	// Test: multiple snapshots in one file
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    insta::assert_snapshot!("a");
    insta::assert_snapshot!("b", @"");
    insta::assert_debug_snapshot!(vec![1]);
}
"#,
		false,
	)), @r###"
	`assert_snapshot!` must use inline snapshot with `@r""` or `@""`
	`assert_debug_snapshot!` must use inline snapshot with `@r""` or `@""`
	"###);

	// Test: run_assert scans tests/ directory (not just src/)
	// This is a regression test for when tests/ directory was not being scanned
	{
		let temp_dir = std::env::temp_dir().join("codestyle_test_insta_tests_dir");
		std::fs::create_dir_all(temp_dir.join("tests")).unwrap();
		std::fs::write(temp_dir.join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"\n").unwrap();
		std::fs::write(
			temp_dir.join("tests/test.rs"),
			r#"
fn test() {
    insta::assert_snapshot!(output);
}
"#,
		)
		.unwrap();

		// Should return exit code 1 due to violation in tests/
		let opts = RustCheckOptions {
			insta_inline_snapshot: true,
			..Default::default()
		};
		let exit_code = run_assert(&temp_dir, &opts);
		assert_eq!(exit_code, 1, "Should detect violations in tests/ directory");

		std::fs::remove_dir_all(&temp_dir).ok();
	}

	println!("All insta_snapshots tests passed!");
}
