use codestyle::{
	rust_checks::{self, Fix, Violation, impl_follows_type},
	test_fixture::Fixture,
};

fn check_violations(code: &str, expected: &[&str]) {
	let fixture = Fixture::parse(code);
	let temp = fixture.write_to_tempdir();

	let file_infos = rust_checks::collect_rust_files(&temp.root);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| impl_follows_type::check(&info.path, &info.contents, tree))
		.collect();
	let messages: Vec<&str> = violations.iter().map(|v| v.message.as_str()).collect();

	assert_eq!(messages, expected, "Violations mismatch for fixture:\n{code}");
}

fn check_ok(code: &str) {
	check_violations(code, &[]);
}

/// Check that applying fix produces expected result
fn check_fix(before: &str, after: &str) {
	let before_fixture = Fixture::parse(before);
	let after_fixture = Fixture::parse(after);

	let before_temp = before_fixture.write_to_tempdir();

	let file_infos = rust_checks::collect_rust_files(&before_temp.root);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| impl_follows_type::check(&info.path, &info.contents, tree))
		.collect();

	assert!(!violations.is_empty(), "Expected violations to fix, found none");

	// Apply fixes in reverse order
	let mut fixes: Vec<&Fix> = violations.iter().filter_map(|v| v.fix.as_ref()).collect();
	fixes.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

	let content = before_fixture.single_file().text.clone();
	let mut result = content.clone();
	for fix in fixes {
		if fix.start_byte <= result.len() && fix.end_byte <= result.len() {
			result.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
		}
	}

	let expected = after_fixture.single_file().text.as_str();
	assert_eq!(result, expected, "Fix result mismatch");
}

fn main() {
	// impl immediately after struct passes
	check_ok(
		r#"
		struct Foo {
			x: i32,
		}
		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
	);

	// impl with gap triggers violation
	check_violations(
		r#"
		struct Foo {
			x: i32,
		}


		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
		&["`impl Foo` should follow type definition (line 3), but has 2 blank line(s)"],
	);

	// trait impl is exempt (can be anywhere)
	check_ok(
		r#"
		struct Foo;


		impl Default for Foo {
			fn default() -> Self { Foo }
		}
		"#,
	);

	// enum works same as struct
	check_violations(
		r#"
		enum Bar {
			A,
			B,
		}


		impl Bar {
			fn is_a(&self) -> bool { matches!(self, Self::A) }
		}
		"#,
		&["`impl Bar` should follow type definition (line 4), but has 2 blank line(s)"],
	);

	// chained impls (multiple impl blocks)
	check_ok(
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

	// impl for type not defined in file is ignored
	check_ok(
		r#"


		impl String {
			fn custom() {}
		}
		"#,
	);

	// auto-fix relocates impl block immediately after struct (blank lines only)
	check_fix(
		r#"
		struct Foo {
			x: i32,
		}


		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
		r#"
		struct Foo {
			x: i32,
		}
		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
	);

	// auto-fix relocates impl block when other code is in between
	check_fix(
		r#"
		struct Foo {
			x: i32,
		}

		fn unrelated() {}

		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
		r#"
		struct Foo {
			x: i32,
		}
		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}

		fn unrelated() {}
		"#,
	);

	// auto-fix with multiple impl blocks for same struct
	check_fix(
		r#"
		struct Foo;

		fn other() {}

		impl Foo {
			fn one() {}
		}

		impl Foo {
			fn two() {}
		}
		"#,
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}

		fn other() {}

		impl Foo {
			fn two() {}
		}
		"#,
	);

	println!("All impl_follows_type tests passed!");
}
