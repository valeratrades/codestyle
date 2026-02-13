//! Tests for codestyle::skip comment markers - skipping codestyle checks on annotated items.
//!
//! Supported formats for skipping all rules:
//! - `//#[codestyle::skip]`
//! - `// #[codestyle::skip]`
//! - `//@codestyle::skip`
//! - `// @codestyle::skip`
//!
//! Supported formats for skipping specific rules:
//! - `//#[codestyle::skip(rule-name)]`
//! - `// #[codestyle::skip(rule-name)]`
//! - `//@codestyle::skip(rule-name)`
//! - `// @codestyle::skip(rule-name)`

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

// === codestyle::skip on functions ===

#[test]
fn skip_on_function_ignores_ignored_error_comment() {
	// A function with //@codestyle::skip should not trigger ignored_error_comment violations
	assert_check_passing(
		r#"
		//@codestyle::skip
		fn skipped() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

#[test]
fn skip_on_function_ignores_loops() {
	// A function with //@codestyle::skip should not trigger loop violations
	assert_check_passing(
		r#"
		//@codestyle::skip
		fn skipped() {
			loop {
				// endless loop without //LOOP comment should be ignored
			}
		}
		"#,
		&opts_for("loops"),
	);
}

#[test]
fn skip_on_function_ignores_embed_simple_vars() {
	// A function with // #[codestyle::skip] should not trigger embed_simple_vars violations
	assert_check_passing(
		r#"
		// #[codestyle::skip]
		fn skipped() {
			let name = "world";
			println!("Hello, {}", name);
		}
		"#,
		&opts_for("embed_simple_vars"),
	);
}

// === codestyle::skip on struct/impl blocks ===

#[test]
fn skip_on_impl_ignores_impl_follows_type() {
	// An impl with //#[codestyle::skip] should not trigger impl_follows_type violations
	assert_check_passing(
		r#"
		//#[codestyle::skip]
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
	// A struct block with // @codestyle::skip should not check pub_first ordering
	assert_check_passing(
		r#"
		// @codestyle::skip
		struct Config {
			private_field: i32,
			pub public_field: i32,
		}
		"#,
		&opts_for("pub_first"),
	);
}

// === codestyle::skip on modules ===

#[test]
fn skip_on_module_ignores_all_inside() {
	// A module with //@codestyle::skip should skip all checks for items inside
	assert_check_passing(
		r#"
		//@codestyle::skip
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

// === Skip with attributes above ===

#[test]
fn skip_between_attribute_and_item_is_detected() {
	// A skip comment placed between #[cfg(test)] and `mod tests` should work
	assert_check_passing(
		r#"
		#[cfg(test)]
		//#[codestyle::skip(sequential-asserts)]
		mod tests {
			fn test_a() {
				assert_eq!(1, 1);
				assert_eq!(2, 2);
			}
		}
		"#,
		&opts_for("insta_inline_snapshot"),
	);
}

#[test]
fn skip_above_attribute_is_detected() {
	// A skip comment placed above #[cfg(test)] should also work
	assert_check_passing(
		r#"
		//#[codestyle::skip(sequential-asserts)]
		#[cfg(test)]
		mod tests {
			fn test_a() {
				assert_eq!(1, 1);
				assert_eq!(2, 2);
			}
		}
		"#,
		&opts_for("insta_inline_snapshot"),
	);
}

// === Edge cases ===

#[test]
fn skip_does_not_affect_sibling_items() {
	// Skip on one function should not affect sibling functions
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		//@codestyle::skip
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
		//@codestyle::skip
		fn outer() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

#[test]
fn all_skip_comment_variants_work() {
	// Test all four supported syntaxes
	assert_check_passing(
		r#"
		//#[codestyle::skip]
		fn skipped1() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}

		// #[codestyle::skip]
		fn skipped2() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}

		//@codestyle::skip
		fn skipped3() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}

		// @codestyle::skip
		fn skipped4() {
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

// === Rule-specific skip ===

#[test]
fn skip_specific_rule_only_skips_that_rule() {
	// skip(ignored-error-comment) should skip that rule but still check others
	assert_check_passing(
		r#"
		//#[codestyle::skip(ignored-error-comment)]
		fn skipped_unwrap() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

#[test]
fn skip_specific_rule_does_not_affect_other_rules() {
	// skip(pub-first) should not skip ignored-error-comment
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		//#[codestyle::skip(pub-first)]
		fn not_skipped_for_unwrap() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	), @"
	[ignored-error-comment] /main.rs:4: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}

#[test]
fn skip_specific_rule_at_syntax() {
	// @codestyle::skip(rule) syntax should also work
	assert_check_passing(
		r#"
		//@codestyle::skip(ignored-error-comment)
		fn skipped() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

#[test]
fn skip_specific_rule_with_spaces() {
	// Spaces inside parens should be trimmed
	assert_check_passing(
		r#"
		// #[codestyle::skip( ignored-error-comment )]
		fn skipped() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts_for("ignored_error_comment"),
	);
}

#[test]
fn skip_specific_rule_pub_first() {
	// skip(pub-first) should skip pub-first check
	assert_check_passing(
		r#"
		//#[codestyle::skip(pub-first)]
		fn private_fn() {}
		pub fn public_fn() {}
		"#,
		&opts_for("pub_first"),
	);
}

#[test]
fn skip_specific_rule_loop_comment() {
	// skip(loop-comment) should skip loop check
	assert_check_passing(
		r#"
		//#[codestyle::skip(loop-comment)]
		fn endless() {
			loop {
				// no LOOP comment needed
			}
		}
		"#,
		&opts_for("loops"),
	);
}

#[test]
fn skip_all_still_works_with_parens_style() {
	// Just checking skip-all still works after adding rule-specific support
	assert_check_passing(
		r#"
		//#[codestyle::skip]
		fn skipped_all() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
			loop {}
		}
		"#,
		&all_opts(),
	);
}
