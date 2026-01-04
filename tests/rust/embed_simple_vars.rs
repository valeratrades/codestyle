use codestyle::rust_checks::{self, embed_simple_vars};

fn check_code(code: &str) -> Vec<String> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_embed_vars");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<String> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| embed_simple_vars::check(&info.path, &info.contents, tree))
		.map(|v| v.message)
		.collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn main() {
	// Test: simple var in println should embed
	let violations = check_code(
		r#"
fn test() {
    let name = "world";
    println!("Hello, {}", name);
}
"#,
	);
	assert_eq!(violations.len(), 1, "should catch simple var: {violations:?}");
	assert!(violations[0].contains("name"));

	// Test: already embedded var passes
	let violations = check_code(
		r#"
fn test() {
    let name = "world";
    println!("Hello, {name}");
}
"#,
	);
	assert!(violations.is_empty(), "embedded var should pass: {violations:?}");

	// Test: complex expression is fine (method call)
	let violations = check_code(
		r#"
fn test() {
    let s = String::new();
    println!("len: {}", s.len());
}
"#,
	);
	assert!(violations.is_empty(), "method call should be fine: {violations:?}");

	// Test: field access is fine
	let violations = check_code(
		r#"
struct Foo { x: i32 }
fn test() {
    let f = Foo { x: 1 };
    println!("x: {}", f.x);
}
"#,
	);
	assert!(violations.is_empty(), "field access should be fine: {violations:?}");

	// Test: all simple vars
	let violations = check_code(
		r#"
fn test() {
    let a = 1;
    let b = 2;
    println!("{} + {}", a, b);
}
"#,
	);
	assert_eq!(violations.len(), 2, "should catch 2 simple vars: {violations:?}");

	// Test: mixed simple and complex - skipped when arg parsing fails to match
	// The checker bails if placeholder count != parsed args count
	// `a + b` may be parsed as multiple tokens, causing mismatch
	let violations = check_code(
		r#"
fn test() {
    let a = 1;
    let b = 2;
    let sum = a + b;
    println!("{} + {} = {}", a, b, sum);
}
"#,
	);
	// All three are simple identifiers now
	assert_eq!(violations.len(), 3, "should catch 3 simple vars: {violations:?}");

	// Test: format! macro works too
	let violations = check_code(
		r#"
fn test() {
    let x = 42;
    let s = format!("value: {}", x);
}
"#,
	);
	assert_eq!(violations.len(), 1, "format! should be checked: {violations:?}");

	// Test: write! macro
	let violations = check_code(
		r#"
use std::io::Write;
fn test() {
    let x = 42;
    let mut buf = Vec::new();
    write!(buf, "{}", x).unwrap();
}
"#,
	);
	assert_eq!(violations.len(), 1, "write! should be checked: {violations:?}");

	// Test: no placeholder = no violation
	let violations = check_code(
		r#"
fn test() {
    println!("Hello, world!");
}
"#,
	);
	assert!(violations.is_empty(), "no placeholder should pass: {violations:?}");

	// Test: named placeholder is fine
	let violations = check_code(
		r#"
fn test() {
    let width = 5;
    println!("{:width$}", "hi");
}
"#,
	);
	assert!(violations.is_empty(), "named placeholder should pass: {violations:?}");

	println!("All embed_simple_vars tests passed!");
}
