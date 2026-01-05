use codestyle::{
	rust_checks::{self, Fix, Violation, join_split_impls},
	test_fixture::Fixture,
};

fn check_violations(code: &str, expected: &[&str]) {
	let fixture = Fixture::parse(code);
	let temp = fixture.write_to_tempdir();

	let file_infos = rust_checks::collect_rust_files(&temp.root);
	let violations: Vec<Violation> = file_infos
		.iter()
		.filter_map(|info| info.syntax_tree.as_ref().map(|tree| (info, tree)))
		.flat_map(|(info, tree)| join_split_impls::check(&info.path, &info.contents, tree))
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
		.flat_map(|(info, tree)| join_split_impls::check(&info.path, &info.contents, tree))
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
	// single impl block passes (no split)
	check_ok(
		r#"
		struct Foo {
			x: i32,
		}
		impl Foo {
			fn new() -> Self { Self { x: 0 } }
			fn get(&self) -> i32 { self.x }
		}
		"#,
	);

	// two impl blocks for same type should be joined
	check_violations(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}
		impl Foo {
			fn two() {}
		}
		"#,
		&["split `impl Foo` blocks should be joined into one"],
	);

	// trait impl is NOT joined with inherent impl
	check_ok(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}
		impl Default for Foo {
			fn default() -> Self { Foo }
		}
		"#,
	);

	// different trait impls are NOT joined
	check_ok(
		r#"
		struct Foo;
		impl Default for Foo {
			fn default() -> Self { Foo }
		}
		impl Clone for Foo {
			fn clone(&self) -> Self { Foo }
		}
		"#,
	);

	// impl blocks for different types are NOT joined
	check_ok(
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
	);

	// auto-fix joins two consecutive impl blocks
	check_fix(
		r#"
		struct Foo;
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
			fn two() {}
		}
		"#,
	);

	// auto-fix joins impl blocks with code in between
	check_fix(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}

		fn unrelated() {}

		impl Foo {
			fn two() {}
		}
		"#,
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
			fn two() {}
		}

		fn unrelated() {}
		"#,
	);

	// auto-fix joins three impl blocks
	check_fix(
		r#"
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
		"#,
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
			fn two() {}
			fn three() {}
		}
		"#,
	);

	println!("All join_split_impls tests passed!");
}
