//! Tests for #[codestyle::skip] attribute - skipping codestyle checks on annotated items.

use codestyle::rust_checks::RustCheckOptions;

use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn all_opts() -> RustCheckOptions {
	RustCheckOptions {
		instrument: false,
		loops: true,
		join_split_impls: true,
		impl_folds: false,
		impl_follows_type: true,
		embed_simple_vars: true,
		insta_inline_snapshot: false,
		no_chrono: true,
		no_tokio_spawn: true,
		use_bail: true,
		test_fn_prefix: false,
		pub_first: true,
		ignored_error_comment: true,
	}
}

// === #[codestyle::skip] on functions ===

#[test]
fn skip_on_function_ignores_ignored_error_comment() {
	// A function with #[codestyle::skip] should not trigger ignored_error_comment violations
	assert_check_passing(
		r#"
		#[codestyle::skip]
		fn skipped() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

#[test]
fn skip_on_function_ignores_embed_simple_vars() {
	// A function with #[codestyle::skip] should not trigger embed_simple_vars violations
	assert_check_passing(
		r#"
		#[codestyle::skip]
		fn skipped() {
			let name = "world";
			println!("Hello, {}", name);
		}
		"#,
		&opts_for("embed_simple_vars"),
	);
}

// === #[codestyle::skip] on struct/impl blocks ===

#[test]
fn skip_on_impl_ignores_impl_follows_type() {
	// An impl with #[codestyle::skip] should not trigger impl_follows_type violations
	assert_check_passing(
		r#"
		#[codestyle::skip]
		impl Foo {
			fn new() -> Self { Self }
		}

		struct Foo;
		"#,
		&opts_for("impl_follows_type"),
	);
}

#[test]
fn skip_on_struct_ignores_pub_first() {
	// A struct block with #[codestyle::skip] should not check pub_first ordering
	assert_check_passing(
		r#"
		#[codestyle::skip]
		struct Config {
			private_field: i32,
			pub public_field: i32,
		}
		"#,
		&opts_for("pub_first"),
	);
}

// === #[codestyle::skip] on expressions/blocks ===

#[test]
fn skip_on_block_ignores_all_checks() {
	// When placed on a block expression, all checks inside should be skipped
	assert_check_passing(
		r#"
		fn outer() {
			#[codestyle::skip]
			{
				let x: Option<i32> = None;
				let y = x.unwrap_or(0);
				let _ = some_result();
			}
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&all_opts(),
	);
}

#[test]
fn skip_on_let_statement_ignores_that_statement() {
	// Skip attribute on a specific let statement
	assert_check_passing(
		r#"
		fn foo() {
			let x: Option<i32> = None;
			#[codestyle::skip]
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

// === #[codestyle::skip] on modules ===

#[test]
fn skip_on_module_ignores_all_inside() {
	// A module with #[codestyle::skip] should skip all checks for items inside
	assert_check_passing(
		r#"
		#[codestyle::skip]
		mod skipped_module {
			fn bad() {
				let x: Option<i32> = None;
				let y = x.unwrap_or(0);
				let _ = some_result();
			}
			fn some_result() -> Result<(), ()> { Ok(()) }
		}
		"#,
		&all_opts(),
	);
}

// === Edge cases ===

#[test]
fn skip_does_not_affect_sibling_items() {
	// Skip on one function should not affect sibling functions
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		#[codestyle::skip]
		fn skipped() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}

		fn not_skipped() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	), @"
	[ignored-error-comment] /main.rs:9: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}

#[test]
fn nested_skip_works() {
	// Skip inside a function that is itself skipped (redundant but should work)
	assert_check_passing(
		r#"
		#[codestyle::skip]
		fn outer() {
			#[codestyle::skip]
			{
				let x: Option<i32> = None;
				let y = x.unwrap_or(0);
			}
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

#[test]
fn skip_inner_attribute_on_function_body() {
	// Inner attribute form should also work
	assert_check_passing(
		r#"
		fn skipped() {
			#![codestyle::skip]
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

// === Verify violations still occur without skip ===

#[test]
fn without_skip_violations_are_detected() {
	// Baseline: without skip, violations should be detected
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	), @"
	[ignored-error-comment] /main.rs:3: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}
