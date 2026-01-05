use codestyle::{
	rust_checks::RustCheckOptions,
	test_fixture::{simulate_check, simulate_format},
};

fn opts() -> RustCheckOptions {
	RustCheckOptions {
		join_split_impls: true,
		impl_follows_type: false,
		loops: false,
		embed_simple_vars: false,
		insta_inline_snapshot: false,
		instrument: false,
	}
}

#[test]
fn single_impl_block_passes() {
	insta::assert_snapshot!(simulate_check(
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
	), @"(no violations)");
}

#[test]
fn two_impl_blocks_for_same_type_should_be_joined() {
	insta::assert_snapshot!(simulate_check(
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
	), @"[join-split-impls] /main.rs:5: split `impl Foo` blocks should be joined into one");
}

#[test]
fn trait_impl_not_joined_with_inherent_impl() {
	insta::assert_snapshot!(simulate_check(
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
	), @"(no violations)");
}

#[test]
fn different_trait_impls_not_joined() {
	insta::assert_snapshot!(simulate_check(
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
	), @"(no violations)");
}

#[test]
fn impl_blocks_for_different_types_not_joined() {
	insta::assert_snapshot!(simulate_check(
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
	), @"(no violations)");
}

#[test]
fn autofix_joins_two_consecutive_impl_blocks() {
	insta::assert_snapshot!(simulate_format(
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
	), @r#"
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
	}
	"#);
}

#[test]
fn autofix_joins_impl_blocks_with_code_in_between() {
	insta::assert_snapshot!(simulate_format(
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
	), @r#"
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
	}

	fn unrelated() {}
	"#);
}

#[test]
fn autofix_joins_three_impl_blocks() {
	insta::assert_snapshot!(simulate_format(
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
	), @r#"
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
		fn three() {}
	}
	"#);
}

#[test]
fn cross_file_impl_blocks_not_detected() {
	// Currently NOT detected (single-file scope)
	insta::assert_snapshot!(simulate_check(
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
	), @"(no violations)");
}
