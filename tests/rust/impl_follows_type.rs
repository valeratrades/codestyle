use codestyle::rust_checks::{self, Violation, impl_follows_type};

fn check_code(code: &str) -> Vec<Violation> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_impl_follows");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| impl_follows_type::check(&info.path, tree))
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
	// Test: impl immediately after struct passes
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo {
    x: i32,
}
impl Foo {
    fn new() -> Self { Self { x: 0 } }
}
"#,
	)), @"(no violations)");

	// Test: impl with gap triggers violation
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo {
    x: i32,
}


impl Foo {
    fn new() -> Self { Self { x: 0 } }
}
"#,
	)), @"`impl Foo` should follow type definition (line 4), but has 2 blank line(s)");

	// Test: trait impl is exempt (can be anywhere)
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
struct Foo;


impl Default for Foo {
    fn default() -> Self { Foo }
}
"#,
	)), @"(no violations)");

	// Test: enum works same as struct
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
enum Bar {
    A,
    B,
}


impl Bar {
    fn is_a(&self) -> bool { matches!(self, Self::A) }
}
"#,
	)), @"`impl Bar` should follow type definition (line 5), but has 2 blank line(s)");

	// Test: chained impls (multiple impl blocks)
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
	)), @"(no violations)");

	// Test: impl for type not defined in file is ignored
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"


impl String {
    fn custom() {}
}
"#,
	)), @"(no violations)");

	println!("All impl_follows_type tests passed!");
}
