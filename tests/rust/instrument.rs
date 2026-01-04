use codestyle::rust_checks::{self, Violation, instrument};

fn check_code(code: &str) -> Vec<Violation> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_instrument");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<Violation> = file_infos.iter().flat_map(|info| instrument::check_instrument(info)).collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn check_code_in_file(code: &str, filename: &str) -> Vec<Violation> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_instrument_named");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join(filename);
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<Violation> = file_infos.iter().flat_map(|info| instrument::check_instrument(info)).collect();

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
	// Test: sync function without #[instrument] passes (only async is checked)
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn sync_no_instrument() {
    println!("hello");
}
"#,
	)), @"(no violations)");

	// Test: async function without #[instrument] triggers violation
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
async fn async_no_instrument() {
    println!("hello");
}
"#,
	)), @"No #[instrument] on async fn `async_no_instrument`");

	// Test: async function with #[instrument] passes
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
#[instrument]
async fn with_instrument() {
    println!("hello");
}
"#,
	)), @"(no violations)");

	// Test: main function is exempt (even if async)
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
async fn main() {
    println!("hello");
}
"#,
	)), @"(no violations)");

	// Test: async functions in utils.rs are exempt
	insta::assert_snapshot!(snapshot_violations(&check_code_in_file(
		r#"
async fn helper() {
    println!("hello");
}
"#,
		"utils.rs",
	)), @"(no violations)");

	// Test: multiple functions - only async without instrument are caught
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn sync_one() {}
async fn async_one() {}
async fn async_two() {}
#[instrument]
async fn async_three() {}
"#,
	)), @r"
	No #[instrument] on async fn `async_one`
	No #[instrument] on async fn `async_two`
	");

	println!("All instrument tests passed!");
}
