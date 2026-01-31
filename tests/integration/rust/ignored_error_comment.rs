use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("ignored_error_comment")
}

// === unwrap_or passing cases ===

#[test]
fn unwrap_or_with_inline_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0); //IGNORED_ERROR: default for missing config
		}
		"#,
		&opts(),
	);
}

#[test]
fn unwrap_or_with_comment_on_line_above_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Option<i32> = None;
			//IGNORED_ERROR: default for missing config
			let y = x.unwrap_or(0);
		}
		"#,
		&opts(),
	);
}

#[test]
fn unwrap_or_default_with_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Option<String> = None;
			let y = x.unwrap_or_default(); //IGNORED_ERROR: empty string is fine
		}
		"#,
		&opts(),
	);
}

#[test]
fn unwrap_or_else_with_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Option<i32> = None;
			let y = x.unwrap_or_else(|| 42); //IGNORED_ERROR: lazy default
		}
		"#,
		&opts(),
	);
}

#[test]
fn result_unwrap_or_with_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Result<i32, ()> = Err(());
			let y = x.unwrap_or(0); //IGNORED_ERROR: fallback on error
		}
		"#,
		&opts(),
	);
}

// === let _ passing cases ===

#[test]
fn let_underscore_with_inline_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let _ = some_result(); //IGNORED_ERROR: intentionally ignoring return value
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	);
}

#[test]
fn let_underscore_with_comment_on_line_above_passes() {
	assert_check_passing(
		r#"
		fn good() {
			//IGNORED_ERROR: we don't care about this result
			let _ = some_result();
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	);
}

#[test]
fn named_underscore_binding_no_comment_needed() {
	assert_check_passing(
		r#"
		fn good() {
			let _unused = 42;
		}
		"#,
		&opts(),
	);
}

#[test]
fn destructuring_with_underscore_no_comment_needed() {
	assert_check_passing(
		r#"
		fn good() {
			let (a, _) = (1, 2);
			let Foo { x, .. } = Foo { x: 1, y: 2 };
		}
		struct Foo { x: i32, y: i32 }
		"#,
		&opts(),
	);
}

// === unwrap_or violation cases ===

#[test]
fn unwrap_or_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:3: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}

#[test]
fn unwrap_or_default_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let x: Option<String> = None;
			let y = x.unwrap_or_default();
		}
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:3: `unwrap_or_default` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}

#[test]
fn unwrap_or_else_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let x: Option<i32> = None;
			let y = x.unwrap_or_else(|| 42);
		}
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:3: `unwrap_or_else` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}

#[test]
fn nested_unwrap_or_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn nested() {
			if true {
				let x: Option<i32> = None;
				let y = x.unwrap_or(0);
			}
		}
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:4: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}

#[test]
fn unwrap_or_in_closure() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn with_closure() {
			let f = || {
				let x: Option<i32> = None;
				x.unwrap_or(0)
			};
		}
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:4: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}

#[test]
fn chained_unwrap_or_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn chained() {
			let x: Option<Option<i32>> = None;
			let y = x.unwrap_or(None).unwrap_or(0);
		}
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:3: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	[ignored-error-comment] /main.rs:3: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option.
	");
}

// === let _ violation cases ===

#[test]
fn let_underscore_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let _ = some_result();
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:2: `let _ = ...` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn nested_let_underscore_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn nested() {
			if true {
				let _ = some_result();
			}
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:3: `let _ = ...` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn let_underscore_in_closure() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn with_closure() {
			let f = || {
				let _ = some_result();
			};
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	), @"
	[ignored-error-comment] /main.rs:3: `let _ = ...` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}
