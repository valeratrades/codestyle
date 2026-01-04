use codestyle::rust_checks::{self, impl_follows_type};

fn check_code(code: &str) -> Vec<String> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_impl_follows");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<String> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| impl_follows_type::check(&info.path, tree))
		.map(|v| v.message)
		.collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn main() {
	// Test: impl immediately after struct passes
	let violations = check_code(
		r#"
struct Foo {
    x: i32,
}
impl Foo {
    fn new() -> Self { Self { x: 0 } }
}
"#,
	);
	assert!(violations.is_empty(), "immediate impl should pass: {violations:?}");

	// Test: impl with gap triggers violation
	let violations = check_code(
		r#"
struct Foo {
    x: i32,
}


impl Foo {
    fn new() -> Self { Self { x: 0 } }
}
"#,
	);
	assert_eq!(violations.len(), 1, "gap should trigger: {violations:?}");
	assert!(violations[0].contains("blank line"));

	// Test: trait impl is exempt (can be anywhere)
	let violations = check_code(
		r#"
struct Foo;


impl Default for Foo {
    fn default() -> Self { Foo }
}
"#,
	);
	assert!(violations.is_empty(), "trait impl should be exempt: {violations:?}");

	// Test: enum works same as struct
	let violations = check_code(
		r#"
enum Bar {
    A,
    B,
}


impl Bar {
    fn is_a(&self) -> bool { matches!(self, Self::A) }
}
"#,
	);
	assert_eq!(violations.len(), 1, "enum gap should trigger: {violations:?}");

	// Test: chained impls (multiple impl blocks)
	let violations = check_code(
		r#"
struct Foo;
impl Foo {
    fn one() {}
}
impl Foo {
    fn two() {}
}
"#,
	);
	assert!(violations.is_empty(), "chained impls should pass: {violations:?}");

	// Test: impl for type not defined in file is ignored
	let violations = check_code(
		r#"


impl String {
    fn custom() {}
}
"#,
	);
	assert!(violations.is_empty(), "external type impl should be ignored: {violations:?}");

	println!("All impl_follows_type tests passed!");
}
