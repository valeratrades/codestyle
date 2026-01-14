use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("join_split_impls")
}

// === Passing cases ===

#[test]
fn single_impl_block_passes() {
	assert_check_passing(
		r#"
		struct Foo {
			x: i32,
		}
		impl Foo {
			fn new() -> Self { Self { x: 0 } }
			fn get(&self) -> i32 { self.x }
		}
		"#,
		&opts(),
	);
}

#[test]
fn trait_impl_not_joined_with_inherent_impl() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}
		impl Default for Foo {
			fn default() -> Self { Foo }
		}
		"#,
		&opts(),
	);
}

#[test]
fn different_trait_impls_not_joined() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Default for Foo {
			fn default() -> Self { Foo }
		}
		impl Clone for Foo {
			fn clone(&self) -> Self { Foo }
		}
		"#,
		&opts(),
	);
}

#[test]
fn impl_blocks_for_different_types_not_joined() {
	assert_check_passing(
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
		&opts(),
	);
}

#[test]
fn cross_file_impl_blocks_not_detected() {
	// Currently NOT detected (single-file scope)
	assert_check_passing(
		r#"
		//- /src/first.rs
		pub struct Foo;
		impl Foo {
			fn bar() {}
		}

		//- /src/second.rs
		use crate::first::Foo;
		impl Foo {
			fn yuck() {
				println!("Cross-file impl - not detected");
			}
		}
		"#,
		&opts(),
	);
}

// === Violation cases ===

#[test]
fn two_consecutive_impl_blocks() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}
		impl Foo {
			fn two() {}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:5: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
	}
	");
}

#[test]
fn impl_blocks_with_code_in_between() {
	insta::assert_snapshot!(test_case(
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
		&opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:8: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
	}

	fn unrelated() {}
	");
}

#[test]
fn three_impl_blocks() {
	insta::assert_snapshot!(test_case(
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
		&opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:5: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
		fn three() {}
	}
	");
}
