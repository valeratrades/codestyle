use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("prefer_default")
}

// === Passing cases ===

#[test]
fn new_with_args_passes() {
	assert_check_passing(
		r#"
		struct Foo { x: i32 }
		impl Foo {
			pub fn new(x: i32) -> Self {
				Self { x }
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn private_new_passes() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo {
			fn new() -> Self {
				Self
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn default_impl_passes() {
	assert_check_passing(
		r#"
		#[derive(Default)]
		struct Foo { x: i32 }
		"#,
		&opts(),
	);
}

#[test]
fn trait_impl_new_passes() {
	// `fn new()` inside a trait impl should not be flagged
	assert_check_passing(
		r#"
		trait Builder {
			fn new() -> Self;
		}
		struct Foo;
		impl Builder for Foo {
			fn new() -> Self { Self }
		}
		"#,
		&opts(),
	);
}

// === Violation cases ===

#[test]
fn pub_new_no_args() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		struct Foo;
		impl Foo {
			pub fn new() -> Self {
				Self
			}
		}
		"#,
		&opts(),
	), @"[prefer-default] /main.rs:3: argument-less `fn new()` found; derive or implement `Default` and use it instead (argument-less `new` is bad practice)");
}

#[test]
fn multiple_types_with_argless_new() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		struct Foo;
		struct Bar;
		impl Foo {
			pub fn new() -> Self { Self }
		}
		impl Bar {
			pub fn new() -> Self { Self }
		}
		"#,
		&opts(),
	), @"
	[prefer-default] /main.rs:4: argument-less `fn new()` found; derive or implement `Default` and use it instead (argument-less `new` is bad practice)
	[prefer-default] /main.rs:7: argument-less `fn new()` found; derive or implement `Default` and use it instead (argument-less `new` is bad practice)
	");
}
