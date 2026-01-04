use codestyle::rust_checks::{self, instrument};

fn check_code(code: &str) -> Vec<String> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_instrument");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<String> = file_infos.iter().flat_map(|info| instrument::check_instrument(info)).map(|v| v.message).collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn check_code_in_file(code: &str, filename: &str) -> Vec<String> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_instrument_named");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join(filename);
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<String> = file_infos.iter().flat_map(|info| instrument::check_instrument(info)).map(|v| v.message).collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn main() {
	// Test: function without #[instrument] triggers violation
	let violations = check_code(
		r#"
fn no_instrument() {
    println!("hello");
}
"#,
	);
	assert_eq!(violations.len(), 1);
	assert!(violations[0].contains("no_instrument"));

	// Test: function with #[instrument] passes
	let violations = check_code(
		r#"
#[instrument]
fn with_instrument() {
    println!("hello");
}
"#,
	);
	assert!(violations.is_empty(), "instrumented fn should pass: {violations:?}");

	// Test: main function is exempt
	let violations = check_code(
		r#"
fn main() {
    println!("hello");
}
"#,
	);
	assert!(violations.is_empty(), "main should be exempt: {violations:?}");

	// Test: functions in utils.rs are exempt
	let violations = check_code_in_file(
		r#"
fn helper() {
    println!("hello");
}
"#,
		"utils.rs",
	);
	assert!(violations.is_empty(), "utils.rs should be exempt: {violations:?}");

	// Test: multiple functions
	let violations = check_code(
		r#"
fn one() {}
fn two() {}
#[instrument]
fn three() {}
"#,
	);
	assert_eq!(violations.len(), 2, "should catch 2 missing instruments: {violations:?}");

	println!("All instrument tests passed!");
}
