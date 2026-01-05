use codestyle::{
	rust_checks::{self, Violation, embed_simple_vars},
	test_fixture::Fixture,
};

fn check_violations(code: &str, expected: &[&str]) {
	let fixture = Fixture::parse(code);
	let temp = fixture.write_to_tempdir();

	let file_infos = rust_checks::collect_rust_files(&temp.root);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| embed_simple_vars::check(&info.path, &info.contents, tree))
		.collect();
	let messages: Vec<&str> = violations.iter().map(|v| v.message.as_str()).collect();

	assert_eq!(messages, expected, "Violations mismatch for fixture:\n{code}");
}

fn check_ok(code: &str) {
	check_violations(code, &[]);
}

fn check_fix(code: &str, expected_fix: &str) {
	let fixture = Fixture::parse(code);
	let temp = fixture.write_to_tempdir();

	let file_infos = rust_checks::collect_rust_files(&temp.root);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| embed_simple_vars::check(&info.path, &info.contents, tree))
		.collect();

	let fix = violations.first().and_then(|v| v.fix.as_ref()).map(|f| f.replacement.as_str()).unwrap_or("(no fix)");

	assert_eq!(fix, expected_fix, "Fix mismatch for fixture:\n{code}");
}

fn main() {
	// simple var in println should embed
	check_violations(
		r#"
		fn test() {
			let name = "world";
			println!("Hello, {}", name);
		}
		"#,
		&["variable `name` should be embedded in format string: use `{name}` instead of `{}, name`"],
	);

	// already embedded var passes
	check_ok(
		r#"
		fn test() {
			let name = "world";
			println!("Hello, {name}");
		}
		"#,
	);

	// complex expression is fine (method call)
	check_ok(
		r#"
		fn test() {
			let s = String::new();
			println!("len: {}", s.len());
		}
		"#,
	);

	// field access is fine
	check_ok(
		r#"
		struct Foo { x: i32 }
		fn test() {
			let f = Foo { x: 1 };
			println!("x: {}", f.x);
		}
		"#,
	);

	// all simple vars
	check_violations(
		r#"
		fn test() {
			let a = 1;
			let b = 2;
			println!("{} + {}", a, b);
		}
		"#,
		&[
			"variable `a` should be embedded in format string: use `{a}` instead of `{}, a`",
			"variable `b` should be embedded in format string: use `{b}` instead of `{}, b`",
		],
	);

	// mixed simple and complex - all simple vars reported
	check_violations(
		r#"
		fn test() {
			let a = 1;
			let b = 2;
			let sum = a + b;
			println!("{} + {} = {}", a, b, sum);
		}
		"#,
		&[
			"variable `a` should be embedded in format string: use `{a}` instead of `{}, a`",
			"variable `b` should be embedded in format string: use `{b}` instead of `{}, b`",
			"variable `sum` should be embedded in format string: use `{sum}` instead of `{}, sum`",
		],
	);

	// format! macro works too
	check_violations(
		r#"
		fn test() {
			let x = 42;
			let s = format!("value: {}", x);
		}
		"#,
		&["variable `x` should be embedded in format string: use `{x}` instead of `{}, x`"],
	);

	// write! macro
	check_violations(
		r#"
		use std::io::Write;
		fn test() {
			let x = 42;
			let mut buf = Vec::new();
			write!(buf, "{}", x).unwrap();
		}
		"#,
		&["variable `x` should be embedded in format string: use `{x}` instead of `{}, x`"],
	);

	// no placeholder = no violation
	check_ok(
		r#"
		fn test() {
			println!("Hello, world!");
		}
		"#,
	);

	// named placeholder is fine
	check_ok(
		r#"
		fn test() {
			let width = 5;
			println!("{:width$}", "hi");
		}
		"#,
	);

	// multi-line format macro should be detected
	check_violations(
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
		&[
			"variable `name` should be embedded in format string: use `{name}` instead of `{}, name`",
			"variable `count` should be embedded in format string: use `{count}` instead of `{}, count`",
		],
	);

	// multi-line format macro fix should be generated
	check_fix(
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
		r#""Hello {name}, you have {count} messages""#,
	);

	// mixed simple and complex args should still fix the simple ones
	check_violations(
		r#"
		fn test() {
			let tf = "1d";
			let s = format!("{}_{}", Utc::now().format("%Y/%m/%d"), tf);
		}
		"#,
		&["variable `tf` should be embedded in format string: use `{tf}` instead of `{}, tf`"],
	);

	// Verify fix is generated for mixed args
	check_fix(
		r#"
		fn test() {
			let tf = "1d";
			let s = format!("{}_{}", Utc::now().format("%Y/%m/%d"), tf);
		}
		"#,
		r#""{}_{tf}", Utc::now().format("%Y/%m/%d")"#,
	);

	// multiple simple vars mixed with complex should fix all simple ones
	check_violations(
		r#"
		fn test() {
			let issue_number = 123;
			let sanitized = "foo";
			let s = format!("{}_-_{}.{}", issue_number, sanitized, extension.as_str());
		}
		"#,
		&[
			"variable `issue_number` should be embedded in format string: use `{issue_number}` instead of `{}, issue_number`",
			"variable `sanitized` should be embedded in format string: use `{sanitized}` instead of `{}, sanitized`",
		],
	);

	// Verify fix embeds both simple vars
	check_fix(
		r#"
		fn test() {
			let issue_number = 123;
			let sanitized = "foo";
			let s = format!("{}_-_{}.{}", issue_number, sanitized, extension.as_str());
		}
		"#,
		r#""{issue_number}_-_{sanitized}.{}", extension.as_str()"#,
	);

	// field access should NOT be doubled
	check_fix(
		r#"
		fn test() {
			let workspace_id = "ws123";
			let s = format!("{}/user/{}/time-entries", workspace_id, user.id);
		}
		"#,
		r#""{workspace_id}/user/{}/time-entries", user.id"#,
	);

	println!("All embed_simple_vars tests passed!");
}
