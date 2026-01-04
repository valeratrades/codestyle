use codestyle::rust_checks::{self, Fix, Violation, join_split_impls};

fn check_code(code: &str) -> Vec<Violation> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_join_split_impls");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| join_split_impls::check(&info.path, &info.contents, tree))
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

fn apply_fix(code: &str, fix: &Fix) -> String {
	let mut result = code.to_string();
	result.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
	result
}

fn apply_all_fixes(code: &str, violations: &[Violation]) -> String {
	let mut fixes: Vec<&Fix> = violations.iter().filter_map(|v| v.fix.as_ref()).collect();
	fixes.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));
	let mut result = code.to_string();
	for fix in fixes {
		if fix.start_byte <= result.len() && fix.end_byte <= result.len() {
			result.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
		}
	}
	result
}

fn main() {
	// Test: single impl block passes (no split)
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo {
    x: i32,
}
impl Foo {
    fn new() -> Self { Self { x: 0 } }
    fn get(&self) -> i32 { self.x }
}
"#,
	)), @"(no violations)");

	// Test: two impl blocks for same type should be joined
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo;
impl Foo {
    fn one() {}
}
impl Foo {
    fn two() {}
}
"#,
	)), @"split `impl Foo` blocks should be joined into one");

	// Test: trait impl is NOT joined with inherent impl
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo;
impl Foo {
    fn one() {}
}
impl Default for Foo {
    fn default() -> Self { Foo }
}
"#,
	)), @"(no violations)");

	// Test: different trait impls are NOT joined
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo;
impl Default for Foo {
    fn default() -> Self { Foo }
}
impl Clone for Foo {
    fn clone(&self) -> Self { Foo }
}
"#,
	)), @"(no violations)");

	// Test: impl blocks for different types are NOT joined
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo;
struct Bar;
impl Foo {
    fn foo() {}
}
impl Bar {
    fn bar() {}
}
"#,
	)), @"(no violations)");

	// Test: auto-fix joins two consecutive impl blocks
	{
		let code = r#"
struct Foo;
impl Foo {
    fn one() {}
}
impl Foo {
    fn two() {}
}
"#;
		let violations = check_code(code);
		assert!(violations.len() == 1, "expected 1 violation");
		assert!(violations[0].fix.is_some(), "expected fix to be present");
		let fixed = apply_fix(code, violations[0].fix.as_ref().unwrap());
		insta::assert_snapshot!(fixed, @r"
struct Foo;
impl Foo {
    fn one() {}
    fn two() {}
}
");
	}

	// Test: auto-fix joins impl blocks with code in between
	{
		let code = r#"
struct Foo;
impl Foo {
    fn one() {}
}

fn unrelated() {}

impl Foo {
    fn two() {}
}
"#;
		let violations = check_code(code);
		assert!(violations.len() == 1, "expected 1 violation");
		let fixed = apply_fix(code, violations[0].fix.as_ref().unwrap());
		insta::assert_snapshot!(fixed, @r"
struct Foo;
impl Foo {
    fn one() {}
    fn two() {}
}

fn unrelated() {}
");
	}

	// Test: auto-fix joins three impl blocks
	{
		let code = r#"
struct Foo;
impl Foo {
    fn one() {}
}
impl Foo {
    fn two() {}
}
impl Foo {
    fn three() {}
}
"#;
		let violations = check_code(code);
		// May report multiple violations, apply all fixes
		let fixed = apply_all_fixes(code, &violations);
		insta::assert_snapshot!(fixed, @r"
struct Foo;
impl Foo {
    fn one() {}
    fn two() {}
    fn three() {}
}
");
	}

	println!("All join_split_impls tests passed!");
}
