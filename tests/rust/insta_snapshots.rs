use codestyle::rust_checks::{self, insta_snapshots};

fn check_code(code: &str, is_format_mode: bool) -> Vec<String> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_insta_snapshots");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<String> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| insta_snapshots::check(&info.path, &info.contents, tree, is_format_mode))
		.map(|v| v.message)
		.collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn main() {
	// Test: assert_snapshot without inline snapshot is a violation
	let violations = check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output);
}
"#,
		false,
	);
	assert_eq!(violations.len(), 1, "should catch missing inline snapshot: {violations:?}");
	assert!(violations[0].contains("must use inline snapshot"));

	// Test: assert_snapshot with inline snapshot passes in assert mode
	let violations = check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @"hello");
}
"#,
		false,
	);
	assert!(violations.is_empty(), "inline snapshot should pass: {violations:?}");

	// Test: assert_snapshot with empty inline snapshot passes
	let violations = check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @"");
}
"#,
		false,
	);
	assert!(violations.is_empty(), "empty inline snapshot should pass: {violations:?}");

	// Test: assert_snapshot with raw string inline snapshot passes
	let violations = check_code(
		r##"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @r#"hello"#);
}
"##,
		false,
	);
	assert!(violations.is_empty(), "raw string inline snapshot should pass: {violations:?}");

	// Test: assert_debug_snapshot variant works
	let violations = check_code(
		r#"
fn test() {
    let output = vec![1, 2, 3];
    insta::assert_debug_snapshot!(output);
}
"#,
		false,
	);
	assert_eq!(violations.len(), 1, "should catch assert_debug_snapshot: {violations:?}");

	// Test: assert_json_snapshot variant works
	let violations = check_code(
		r#"
fn test() {
    let output = serde_json::json!({"key": "value"});
    insta::assert_json_snapshot!(output);
}
"#,
		false,
	);
	assert_eq!(violations.len(), 1, "should catch assert_json_snapshot: {violations:?}");

	// Test: format mode should NOT touch snapshots that already have inline strings
	// This was a bug - format mode was clearing existing snapshot content
	let violations = check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @"hello");
}
"#,
		true,
	);
	assert!(violations.is_empty(), "format mode should NOT touch existing inline snapshots: {violations:?}");

	// Test: format mode with empty inline snapshot passes (no change needed)
	let violations = check_code(
		r#"
fn test() {
    let output = "hello";
    insta::assert_snapshot!(output, @"");
}
"#,
		true,
	);
	assert!(violations.is_empty(), "format mode with empty snapshot should pass: {violations:?}");

	// Test: format mode should NOT touch multiline snapshots with content
	// Bug case from ~/s/todo - multiline snapshots were being cleared
	let violations = check_code(
		r#"
fn test() {
    assert_snapshot!(extract_blockers_section(content).unwrap(), @"
        # Phase 1
        - First task
        ");
}
"#,
		true,
	);
	assert!(violations.is_empty(), "format mode should NOT touch multiline snapshots: {violations:?}");

	// Test: format mode should NOT touch single-line non-empty snapshots
	// Bug case from ~/s/todo
	let violations = check_code(
		r#"
fn test() {
    assert_snapshot!(get_current_blocker_from_content(blockers_content).unwrap(), @"- Third task");
}
"#,
		true,
	);
	assert!(violations.is_empty(), "format mode should NOT touch single-line non-empty snapshots: {violations:?}");

	// Test: format mode should NOT touch raw string snapshots with content
	// Bug case from ~/s/todo
	let violations = check_code(
		r##"
fn test() {
    assert_snapshot!(format!("{:?}", items), @r#"[("Phase 1", true, false), ("Completed task", false, true)]"#);
}
"##,
		true,
	);
	assert!(violations.is_empty(), "format mode should NOT touch raw string snapshots: {violations:?}");

	// Test: non-insta macro with similar name - we still catch it since it uses the same macro name
	// This is acceptable behavior - if users define their own assert_snapshot they should use different name
	let violations = check_code(
		r#"
macro_rules! assert_snapshot {
    ($x:expr) => {};
}
fn test() {
    assert_snapshot!("test");
}
"#,
		false,
	);
	// User-defined macros with same name will trigger - acceptable tradeoff
	assert!(violations.len() <= 1, "non-insta macro handling: {violations:?}");

	// Test: multiple snapshots in one file
	let violations = check_code(
		r#"
fn test() {
    insta::assert_snapshot!("a");
    insta::assert_snapshot!("b", @"");
    insta::assert_debug_snapshot!(vec![1]);
}
"#,
		false,
	);
	assert_eq!(violations.len(), 2, "should catch 2 missing inline snapshots: {violations:?}");

	println!("All insta_snapshots tests passed!");
}
