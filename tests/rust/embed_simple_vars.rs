use codestyle::rust_checks::{self, Violation, embed_simple_vars};

fn check_code(code: &str) -> Vec<Violation> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_embed_vars");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| embed_simple_vars::check(&info.path, &info.contents, tree))
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

fn snapshot_fix(violations: &[Violation]) -> String {
	violations
		.first()
		.and_then(|v| v.fix.as_ref())
		.map(|f| f.replacement.clone())
		.unwrap_or_else(|| "(no fix)".to_string())
}

fn main() {
	// Test: simple var in println should embed
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let name = "world";
    println!("Hello, {}", name);
}
"#,
	)), @"variable `name` should be embedded in format string: use `{name}` instead of `{}, name`");

	// Test: already embedded var passes
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let name = "world";
    println!("Hello, {name}");
}
"#,
	)), @"(no violations)");

	// Test: complex expression is fine (method call)
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let s = String::new();
    println!("len: {}", s.len());
}
"#,
	)), @"(no violations)");

	// Test: field access is fine
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo { x: i32 }
fn test() {
    let f = Foo { x: 1 };
    println!("x: {}", f.x);
}
"#,
	)), @"(no violations)");

	// Test: all simple vars
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let a = 1;
    let b = 2;
    println!("{} + {}", a, b);
}
"#,
	)), @r"
	variable `a` should be embedded in format string: use `{a}` instead of `{}, a`
	variable `b` should be embedded in format string: use `{b}` instead of `{}, b`
	");

	// Test: mixed simple and complex - all simple vars reported
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let a = 1;
    let b = 2;
    let sum = a + b;
    println!("{} + {} = {}", a, b, sum);
}
"#,
	)), @r"
	variable `a` should be embedded in format string: use `{a}` instead of `{}, a`
	variable `b` should be embedded in format string: use `{b}` instead of `{}, b`
	variable `sum` should be embedded in format string: use `{sum}` instead of `{}, sum`
	");

	// Test: format! macro works too
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let x = 42;
    let s = format!("value: {}", x);
}
"#,
	)), @"variable `x` should be embedded in format string: use `{x}` instead of `{}, x`");

	// Test: write! macro
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
use std::io::Write;
fn test() {
    let x = 42;
    let mut buf = Vec::new();
    write!(buf, "{}", x).unwrap();
}
"#,
	)), @"variable `x` should be embedded in format string: use `{x}` instead of `{}, x`");

	// Test: no placeholder = no violation
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    println!("Hello, world!");
}
"#,
	)), @"(no violations)");

	// Test: named placeholder is fine
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let width = 5;
    println!("{:width$}", "hi");
}
"#,
	)), @"(no violations)");

	// Test: multi-line format macro should be detected
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let name = "world";
    let count = 42;
    println!(
        "Hello {}, you have {} messages",
        name,
        count
    );
}
"#,
	)), @r"
	variable `name` should be embedded in format string: use `{name}` instead of `{}, name`
	variable `count` should be embedded in format string: use `{count}` instead of `{}, count`
	");

	// Test: multi-line format macro fix should be generated
	insta::assert_snapshot!(snapshot_fix(&check_code(
		r#"
fn test() {
    let name = "world";
    let count = 42;
    println!(
        "Hello {}, you have {} messages",
        name,
        count
    );
}
"#,
	)), @r#""Hello {name}, you have {count} messages""#);

	// Test: mixed simple and complex args should still fix the simple ones
	// Bug: previously only fixed when ALL args were simple
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let tf = "1d";
    let s = format!("{}_{}", Utc::now().format("%Y/%m/%d"), tf);
}
"#,
	)), @"variable `tf` should be embedded in format string: use `{tf}` instead of `{}, tf`");

	// Verify fix is generated for mixed args
	insta::assert_snapshot!(snapshot_fix(&check_code(
		r#"
fn test() {
    let tf = "1d";
    let s = format!("{}_{}", Utc::now().format("%Y/%m/%d"), tf);
}
"#,
	)), @r#""{}_{tf}", Utc::now().format("%Y/%m/%d")"#);

	// Test: multiple simple vars mixed with complex should fix all simple ones
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn test() {
    let issue_number = 123;
    let sanitized = "foo";
    let s = format!("{}_-_{}.{}", issue_number, sanitized, extension.as_str());
}
"#,
	)), @r"
	variable `issue_number` should be embedded in format string: use `{issue_number}` instead of `{}, issue_number`
	variable `sanitized` should be embedded in format string: use `{sanitized}` instead of `{}, sanitized`
	");

	// Verify fix embeds both simple vars
	insta::assert_snapshot!(snapshot_fix(&check_code(
		r#"
fn test() {
    let issue_number = 123;
    let sanitized = "foo";
    let s = format!("{}_-_{}.{}", issue_number, sanitized, extension.as_str());
}
"#,
	)), @r#""{issue_number}_-_{sanitized}.{}", extension.as_str()"#);

	// Test: field access should NOT be doubled
	// Bug case: format!("...{}/user/{}/...", workspace_id, user.id) was producing user.id.id
	insta::assert_snapshot!(snapshot_fix(&check_code(
		r#"
fn test() {
    let workspace_id = "ws123";
    let s = format!("{}/user/{}/time-entries", workspace_id, user.id);
}
"#,
	)), @r#""{workspace_id}/user/{}/time-entries", user.id"#);

	println!("All embed_simple_vars tests passed!");
}
